use crate::app_context::AppContext;
#[cfg(feature = "cli")]
use crate::cli::{RoadsterCli, RunCommand, RunRoadsterCommand};
use crate::config::app_config::AppConfig;
#[cfg(not(feature = "cli"))]
use crate::config::environment::Environment;
#[cfg(feature = "sidekiq")]
use crate::config::worker::StaleCleanUpBehavior;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
#[cfg(feature = "sidekiq")]
use crate::worker::registry::WorkerRegistry;
#[cfg(feature = "sidekiq")]
use anyhow::anyhow;
use async_trait::async_trait;
#[cfg(feature = "cli")]
use clap::{Args, Command, FromArgMatches};
#[cfg(feature = "sidekiq")]
use itertools::Itertools;
#[cfg(feature = "sidekiq")]
use num_traits::ToPrimitive;
#[cfg(feature = "db-sql")]
use sea_orm::{ConnectOptions, Database};
#[cfg(feature = "db-sql")]
use sea_orm_migration::MigratorTrait;
#[cfg(feature = "sidekiq")]
use sidekiq::{periodic, Processor, ProcessorConfig};
use std::future;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
#[cfg(feature = "sidekiq")]
use tracing::debug;
use tracing::{error, info, instrument};

// todo: this method is getting unweildy, we should break it up
pub async fn start<A>(
    // This parameter is (currently) not used when no features are enabled.
    #[allow(unused_variables)] app: A,
) -> anyhow::Result<()>
where
    A: App + Default + Send + Sync + 'static,
{
    #[cfg(feature = "cli")]
    let (roadster_cli, app_cli) = {
        // Build the CLI by augmenting a default Command with both the roadster and app-specific CLIs
        let cli = Command::default();
        // Add the roadster CLI. Save the shared attributes to use after adding the app-specific CLI
        let cli = RoadsterCli::augment_args(cli);
        let about = cli.get_about().cloned();
        let long_about = cli.get_long_about().cloned();
        let version = cli.get_version().map(|x| x.to_string());
        let long_version = cli.get_long_version().map(|x| x.to_string());
        // Add the app-specific CLI. This will override the shared attributes, so we need to
        // combine them with the roadster CLI attributes.
        let cli = A::Cli::augment_args(cli);
        let cli = if let Some((a, b)) = about.zip(cli.get_about().cloned()) {
            cli.about(format!("{a}\n\n{b}"))
        } else {
            cli
        };
        let cli = if let Some((a, b)) = long_about.zip(cli.get_long_about().cloned()) {
            cli.long_about(format!("{a}\n\n{b}"))
        } else {
            cli
        };
        let cli = if let Some((a, b)) = version.zip(cli.get_version().map(|x| x.to_string())) {
            cli.version(format!("roadster: {a}, app: {b}"))
        } else {
            cli
        };
        let cli =
            if let Some((a, b)) = long_version.zip(cli.get_long_version().map(|x| x.to_string())) {
                cli.long_version(format!("roadster: {a}\n\napp: {b}"))
            } else {
                cli
            };
        // Build each CLI from the CLI args
        let matches = cli.get_matches();
        let roadster_cli = RoadsterCli::from_arg_matches(&matches)?;
        let app_cli = A::Cli::from_arg_matches(&matches)?;
        (roadster_cli, app_cli)
    };

    #[cfg(feature = "cli")]
    let environment = roadster_cli.environment.clone();
    #[cfg(not(feature = "cli"))]
    let environment: Option<Environment> = None;

    let config = AppConfig::new(environment)?;

    A::init_tracing(&config)?;

    #[cfg(feature = "db-sql")]
    let db = Database::connect(A::db_connection_options(&config)?).await?;

    #[cfg(feature = "sidekiq")]
    let (redis_enqueue, redis_fetch) = {
        let sidekiq_config = &config.worker.sidekiq;
        let redis_config = &sidekiq_config.redis;
        let redis = sidekiq::RedisConnectionManager::new(redis_config.uri.to_string())?;
        let redis_enqueue = {
            let pool = bb8::Pool::builder().min_idle(redis_config.enqueue_pool.min_idle);
            let pool = redis_config
                .enqueue_pool
                .max_connections
                .iter()
                .fold(pool, |pool, max_conns| pool.max_size(*max_conns));
            pool.build(redis.clone()).await?
        };
        let redis_fetch = if redis_config
            .fetch_pool
            .max_connections
            .iter()
            .any(|max_conns| *max_conns == 0)
        {
            info!(
                "Redis fetch pool configured with size of zero, will not start the Sidekiq processor"
            );
            None
        } else {
            let pool = bb8::Pool::builder().min_idle(redis_config.fetch_pool.min_idle);
            let pool = redis_config
                .fetch_pool
                .max_connections
                .iter()
                .fold(pool, |pool, max_conns| pool.max_size(*max_conns));
            Some(pool.build(redis.clone()).await?)
        };
        (redis_enqueue, redis_fetch)
    };

    let context = AppContext::new(
        config,
        #[cfg(feature = "db-sql")]
        db,
        #[cfg(feature = "sidekiq")]
        redis_enqueue.clone(),
        #[cfg(feature = "sidekiq")]
        redis_fetch.clone(),
    )
    .await?;

    let context = Arc::new(context);
    let state = A::context_to_state(context.clone()).await?;
    let state = Arc::new(state);

    #[cfg(feature = "cli")]
    {
        if roadster_cli.run(&app, &roadster_cli, &context).await? {
            return Ok(());
        }
        if app_cli.run(&app, &app_cli, &state).await? {
            return Ok(());
        }
    }

    let mut service_registry = ServiceRegistry::new(context.clone(), state.clone());
    A::services(&mut service_registry, &context, &state).await?;

    #[cfg(feature = "cli")]
    for (_name, service) in service_registry.services.iter() {
        if service
            .handle_cli(&roadster_cli, &app_cli, &context, &state)
            .await?
        {
            return Ok(());
        }
    }

    #[cfg(feature = "db-sql")]
    if context.config.database.auto_migrate {
        A::M::up(&context.db, None).await?;
    }

    #[cfg(feature = "sidekiq")]
    let (processor, sidekiq_cancellation_token, _sidekiq_cancellation_token_drop_guard) =
        if redis_fetch.is_some() && context.config.worker.sidekiq.queues.is_empty() {
            info!("No Sidekiq queues configured, not starting the Sidekiq processor");
            (None, None, None)
        } else if let Some(redis_fetch) = redis_fetch {
            if context.config.worker.sidekiq.periodic.stale_cleanup
                == StaleCleanUpBehavior::AutoCleanAll
            {
                // Periodic jobs are not removed automatically. Remove any periodic jobs that were
                // previously added. They should be re-added by `App::worker`.
                periodic::destroy_all(redis_enqueue).await?;
            }
            let queues = context
                .config
                .worker
                .sidekiq
                .queues
                .clone()
                .into_iter()
                .chain(A::worker_queues(&context, &state))
                .collect_vec();
            info!(
                "Creating Sidekiq.rs (rusty-sidekiq) processor with {} queues",
                queues.len()
            );
            debug!("Sidekiq.rs queues: {queues:?}");
            let processor = {
                let num_workers = context
                    .config
                    .worker
                    .sidekiq
                    .num_workers
                    .to_usize()
                    .ok_or_else(|| {
                        anyhow!(
                            "Unable to convert num_workers `{}` to usize",
                            context.config.worker.sidekiq.num_workers
                        )
                    })?;
                let processor_config: ProcessorConfig = Default::default();
                let processor_config = processor_config.num_workers(num_workers);
                let processor =
                    Processor::new(redis_fetch, queues.clone()).with_config(processor_config);
                let mut registry = WorkerRegistry::new(processor, state.clone());
                A::workers(&mut registry, &context, &state).await?;
                registry.remove_stale_periodic_jobs(&context).await?;
                registry.processor
            };
            let token = processor.get_cancellation_token();

            (
                Some(processor),
                Some(token.clone()),
                Some(token.drop_guard()),
            )
        } else {
            info!("Not starting the Sidekiq processor");
            (None, None, None)
        };

    let cancel_token = CancellationToken::new();
    let mut join_set = JoinSet::new();

    // Spawn tasks for the app's services
    for (name, service) in service_registry.services {
        let context = context.clone();
        let state = state.clone();
        let cancel_token = cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            info!(service=%name, "Running service");
            service.run(context, state, cancel_token).await
        }));
    }

    // Task to run the sidekiq processor
    #[cfg(feature = "sidekiq")]
    join_set.spawn(Box::pin(async {
        if let Some(processor) = processor {
            processor.run().await;
        }
        Ok(())
    }));

    // Task to clean up resources when gracefully shutting down.
    join_set.spawn(cancel_on_error(
        cancel_token.clone(),
        context.clone(),
        graceful_shutdown(
            token_shutdown_signal(cancel_token.clone()),
            A::graceful_shutdown(context.clone(), state.clone()),
            #[cfg(feature = "db-sql")]
            context.clone(),
            #[cfg(feature = "sidekiq")]
            sidekiq_cancellation_token,
        ),
    ));
    // Task to listen for the signal to gracefully shutdown, and trigger other tasks to stop.
    let graceful_shutdown_signal = graceful_shutdown_signal(
        cancel_token.clone(),
        A::graceful_shutdown_signal(context.clone(), state.clone()),
    );
    join_set.spawn(cancel_token_on_signal_received(
        graceful_shutdown_signal,
        cancel_token.clone(),
    ));

    // Wait for all the tasks to complete.
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(join_ok) => {
                if let Err(err) = join_ok {
                    error!("An error occurred in one of the app's tasks. Error: {err}");
                }
            }
            Err(join_err) => {
                error!(
                    "An error occurred when trying to join on one of the app's tasks. Error: {join_err}"
                );
            }
        }
    }

    info!("Shutdown complete");

    Ok(())
}

#[async_trait]
pub trait App: Send + Sync {
    type State: From<Arc<AppContext>> + Into<Arc<AppContext>> + Clone + Send + Sync + 'static;
    #[cfg(feature = "cli")]
    type Cli: clap::Args + RunCommand<Self>;
    #[cfg(feature = "db-sql")]
    type M: MigratorTrait;

    fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
        init_tracing(config)?;

        Ok(())
    }

    #[cfg(feature = "db-sql")]
    fn db_connection_options(config: &AppConfig) -> anyhow::Result<ConnectOptions> {
        let mut options = ConnectOptions::new(config.database.uri.to_string());
        options
            .connect_timeout(config.database.connect_timeout)
            .acquire_timeout(config.database.acquire_timeout)
            .min_connections(config.database.min_connections)
            .max_connections(config.database.max_connections)
            .sqlx_logging(false);
        if let Some(idle_timeout) = config.database.idle_timeout {
            options.idle_timeout(idle_timeout);
        }
        if let Some(max_lifetime) = config.database.max_lifetime {
            options.max_lifetime(max_lifetime);
        }
        Ok(options)
    }

    /// Convert the [AppContext] to the custom [Self::State] that will be used throughout the app.
    /// The conversion can simply happen in a [`From<AppContext>`] implementation, but this
    /// method is provided in case there's any additional work that needs to be done that the
    /// consumer can't put in a [`From<AppContext>`] implementation. For example, any
    /// configuration that needs to happen in an async method.
    async fn context_to_state(context: Arc<AppContext>) -> anyhow::Result<Self::State> {
        let state = Self::State::from(context);
        Ok(state)
    }

    /// Worker queue names can either be provided here, or as config values. If provided here
    /// the consumer is able to use string constants, which can be used when creating a worker
    /// instance. This can reduce the risk of copy/paste errors and typos.
    #[cfg(feature = "sidekiq")]
    fn worker_queues(_context: &AppContext, _state: &Self::State) -> Vec<String> {
        Default::default()
    }

    #[cfg(feature = "sidekiq")]
    async fn workers(
        _registry: &mut WorkerRegistry<Self>,
        _context: &AppContext,
        _state: &Self::State,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn services(
        _registry: &mut ServiceRegistry<Self>,
        _context: &AppContext,
        _state: &Self::State,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Override to provide a custom shutdown signal. Roadster provides some default shutdown
    /// signals, but it may be desirable to provide a custom signal in order to, e.g., shutdown the
    /// server when a particular API is called.
    async fn graceful_shutdown_signal(_context: Arc<AppContext>, _state: Arc<Self::State>) {
        let _output: () = future::pending().await;
    }

    /// Override to provide custom graceful shutdown logic to clean up any resources created by
    /// the app. Roadster will take care of cleaning up the resources it created.
    #[instrument(skip_all)]
    async fn graceful_shutdown(
        _context: Arc<AppContext>,
        _state: Arc<Self::State>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

async fn graceful_shutdown_signal<F>(cancellation_token: CancellationToken, app_shutdown_signal: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Shutting down due to ctrl-c signal received");
        },
        _ = sigterm => {
            info!("Shutting down due to sigterm signal received");
        },
        _ = cancellation_token.cancelled() => {
            info!("Shutting down due to cancellation token cancelled");
        }
        _ = app_shutdown_signal => {
            info!("Shutting down due to app's custom shutdown signal received");
        }
    }
}

async fn cancel_token_on_signal_received<F>(
    shutdown_signal: F,
    cancellation_token: CancellationToken,
) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    shutdown_signal.await;
    cancellation_token.cancel();
    Ok(())
}

async fn token_shutdown_signal(cancellation_token: CancellationToken) {
    cancellation_token.cancelled().await
}

async fn cancel_on_error<T, F>(
    cancellation_token: CancellationToken,
    context: Arc<AppContext>,
    f: F,
) -> anyhow::Result<T>
where
    F: Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let result = f.await;
    if result.is_err() && context.config.app.shutdown_on_error {
        cancellation_token.cancel();
    }
    result
}

#[instrument(skip_all)]
async fn graceful_shutdown<F1, F2>(
    shutdown_signal: F1,
    app_graceful_shutdown: F2,
    #[cfg(feature = "db-sql")] context: Arc<AppContext>,
    #[cfg(feature = "sidekiq")] sidekiq_cancellation_token: Option<CancellationToken>,
) -> anyhow::Result<()>
where
    F1: Future<Output = ()> + Send + 'static,
    F2: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    shutdown_signal.await;

    info!("Received shutdown signal. Shutting down gracefully.");

    #[cfg(feature = "db-sql")]
    let db_close_result = {
        info!("Closing the DB connection pool.");
        context.as_ref().clone().db.close().await
    };

    #[cfg(feature = "sidekiq")]
    if let Some(sidekiq_cancellation_token) = sidekiq_cancellation_token {
        info!("Cancelling sidekiq workers.");
        sidekiq_cancellation_token.cancel();
    }

    // Futures are lazy -- the custom `app_graceful_shutdown` future won't run until we call `await` on it.
    // https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
    info!("Running app's custom shutdown logic.");
    let app_graceful_shutdown_result = app_graceful_shutdown.await;

    #[cfg(feature = "db-sql")]
    db_close_result?;
    app_graceful_shutdown_result?;

    Ok(())
}
