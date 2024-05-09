use crate::app_context::AppContext;
#[cfg(feature = "cli")]
use crate::cli::{RoadsterCli, RunCommand, RunRoadsterCommand};
use crate::config::app_config::AppConfig;
#[cfg(not(feature = "cli"))]
use crate::config::environment::Environment;
use crate::service::registry::ServiceRegistry;
use crate::tracing::init_tracing;
use async_trait::async_trait;
#[cfg(feature = "cli")]
use clap::{Args, Command, FromArgMatches};
#[cfg(feature = "db-sql")]
use sea_orm::{ConnectOptions, Database};
#[cfg(feature = "db-sql")]
use sea_orm_migration::MigratorTrait;
use std::future;
use std::future::Future;
use std::sync::Arc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, warn};

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
        let sidekiq_config = &config.service.sidekiq;
        let redis_config = &sidekiq_config.custom.redis;
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
    A::services(&mut service_registry, context.clone(), state.clone()).await?;

    #[cfg(feature = "cli")]
    for (_name, service) in service_registry.services.iter() {
        if service
            .handle_cli(&roadster_cli, &app_cli, &context, &state)
            .await?
        {
            return Ok(());
        }
    }

    if service_registry.services.is_empty() {
        warn!("No enabled services were registered, exiting.");
        return Ok(());
    }

    #[cfg(feature = "db-sql")]
    if context.config().database.auto_migrate {
        A::M::up(context.db(), None).await?;
    }

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

    // Task to clean up resources when gracefully shutting down.
    join_set.spawn(cancel_on_error(
        cancel_token.clone(),
        context.clone(),
        graceful_shutdown(
            token_shutdown_signal(cancel_token.clone()),
            A::graceful_shutdown(context.clone(), state.clone()),
            #[cfg(feature = "db-sql")]
            context.clone(),
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

    /// Provide the services to run in the app.
    async fn services(
        _registry: &mut ServiceRegistry<Self>,
        _context: Arc<AppContext>,
        _state: Arc<Self::State>,
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
    if result.is_err() && context.config().app.shutdown_on_error {
        cancellation_token.cancel();
    }
    result
}

#[instrument(skip_all)]
async fn graceful_shutdown<F1, F2>(
    shutdown_signal: F1,
    app_graceful_shutdown: F2,
    #[cfg(feature = "db-sql")] context: Arc<AppContext>,
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
        context.db().clone().close().await
    };

    // Futures are lazy -- the custom `app_graceful_shutdown` future won't run until we call `await` on it.
    // https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
    info!("Running app's custom shutdown logic.");
    let app_graceful_shutdown_result = app_graceful_shutdown.await;

    #[cfg(feature = "db-sql")]
    db_close_result?;
    app_graceful_shutdown_result?;

    Ok(())
}
