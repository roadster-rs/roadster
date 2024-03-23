use std::future;
use std::future::Future;
use std::sync::Arc;

use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use aide::transform::TransformOpenApi;
use async_trait::async_trait;
use axum::{Extension, Router};
use itertools::Itertools;
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::MigratorTrait;
use sidekiq::{periodic, Processor};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{debug, error, info, instrument};

use crate::app_context::AppContext;
use crate::config::app_config::AppConfig;
use crate::controller::middleware::default::default_middleware;
use crate::controller::middleware::Middleware;
use crate::initializer::default::default_initializers;
use crate::initializer::Initializer;
use crate::tracing::init_tracing;
use crate::worker::queue_names;

// todo: this method is getting unweildy, we should break it up
pub async fn start<A, M>() -> anyhow::Result<()>
where
    A: App + Default + Send + Sync + 'static,
    M: MigratorTrait,
{
    let config = AppConfig::new()?;

    A::init_tracing(&config)?;

    debug!("{config:?}");

    let db = Database::connect(A::db_connection_options(&config)?).await?;

    // Todo: enable manual migrations
    if config.database.auto_migrate {
        M::up(&db, None).await?;
    }

    let redis = config.worker.as_ref().map(|worker| &worker.redis);
    let redis = if let Some(redis) = redis {
        let redis = sidekiq::RedisConnectionManager::new(redis.uri.to_string())?;
        let redis = bb8::Pool::builder().build(redis).await?;
        Some(redis)
    } else {
        None
    };

    let mut context = AppContext::new(config, db, redis.clone()).await?;

    let initializers = default_initializers()
        .into_iter()
        .chain(A::initializers(&context))
        .filter(|initializer| initializer.enabled(&context))
        .unique_by(|initializer| initializer.name())
        .sorted_by(|a, b| Ord::cmp(&a.priority(&context), &b.priority(&context)))
        .collect_vec();

    let router = A::router(&context);
    let router = match router {
        RouterType::AxumRouter(router) => router,
        RouterType::AideRouter(router) => {
            let mut api = OpenApi::default();
            let router = router.finish_api_with(&mut api, A::api_docs(&context));
            // Arc is very important here or we will face massive memory and performance issues
            let api = Arc::new(api);
            context.api = Some(api.clone());
            router.layer(Extension(api))
        }
    };
    let context = Arc::new(context);
    let state = A::context_to_state(context.clone()).await?;
    let router = router.with_state::<()>(state.clone());
    let state = Arc::new(state);

    let router = initializers
        .iter()
        .try_fold(router, |router, initializer| {
            initializer.after_router(router, &context)
        })?;

    let router = initializers
        .iter()
        .try_fold(router, |router, initializer| {
            initializer.before_middleware(router, &context)
        })?;

    // Install middleware, both the default middleware and any provided by the consumer.
    info!("Installing middleware. Note: the order of installation is the inverse of the order middleware will run when handling a request.");
    let router = default_middleware()
        .into_iter()
        .chain(A::middleware(&context, &state).into_iter())
        .filter(|middleware| middleware.enabled(&context))
        .unique_by(|middleware| middleware.name())
        .sorted_by(|a, b| Ord::cmp(&a.priority(&context), &b.priority(&context)))
        // Reverse due to how Axum's `Router#layer` method adds middleware.
        .rev()
        .try_fold(router, |router, middleware| {
            info!("Installing middleware: `{}`", middleware.name());
            middleware.install(router, &context)
        })?;

    let router = initializers
        .iter()
        .try_fold(router, |router, initializer| {
            initializer.after_middleware(router, &context)
        })?;

    let router = initializers
        .iter()
        .try_fold(router, |router, initializer| {
            initializer.before_serve(router, &context)
        })?;

    let processor = if let Some(redis) = redis {
        // Periodic jobs are not removed automatically. Remove any periodic jobs that were
        // previously added. They should be re-added by `App::worker`.
        periodic::destroy_all(redis.clone()).await?;
        let custom_queue_names = context
            .config
            .worker
            .as_ref()
            .map(|worker| worker.queue_names.clone())
            .unwrap_or_else(Vec::new)
            .into_iter()
            .chain(A::worker_queues(&context, &state))
            .collect();
        let queue_names = queue_names(&custom_queue_names);
        let mut processor = Processor::new(redis, queue_names);
        A::workers(&mut processor, &context, &state);
        Some(processor)
    } else {
        None
    };
    let sidekiq_cancellation_token = processor
        .as_ref()
        .map(|processor| processor.get_cancellation_token());
    let _sidekiq_cancellation_token_drop_guard = sidekiq_cancellation_token
        .as_ref()
        .map(|token| token.clone().drop_guard());

    let cancel_token = CancellationToken::new();
    let tracker = TaskTracker::new();
    // Task to serve the app.
    tracker.spawn(cancel_on_error(
        cancel_token.clone(),
        context.clone(),
        A::serve(
            router,
            token_shutdown_signal(cancel_token.clone()),
            context.clone(),
            state.clone(),
        ),
    ));
    // Task to run the sidekiq processor
    processor.map(|processor| tracker.spawn(processor.run()));
    // Task to clean up resources when gracefully shutting down.
    tracker.spawn(cancel_on_error(
        cancel_token.clone(),
        context.clone(),
        graceful_shutdown(
            token_shutdown_signal(cancel_token.clone()),
            A::graceful_shutdown(context.clone(), state.clone()),
            sidekiq_cancellation_token,
            context.clone(),
        ),
    ));
    // Task to listen for the signal to gracefully shutdown, and trigger other tasks to stop.
    let graceful_shutdown_signal = graceful_shutdown_signal(
        cancel_token.clone(),
        A::graceful_shutdown_signal(context.clone(), state.clone()),
    );
    tracker.spawn(cancel_token_on_signal_received(
        graceful_shutdown_signal,
        cancel_token.clone(),
    ));

    // Wait for all the tasks to complete.
    tracker.close();
    tracker.wait().await;

    info!("Shutdown complete");

    Ok(())
}

#[async_trait]
pub trait App {
    type State: From<Arc<AppContext>> + Into<Arc<AppContext>> + Clone + Send + Sync + 'static;

    fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
        init_tracing(config)?;

        Ok(())
    }

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
    /// The conversion should mostly happen in a [`From<AppContext>`] implementation, but this
    /// method is provided in case there's any additional work that needs to be done that the
    /// consumer doesn't want to put in a [`From<AppContext>`] implementation. For example, any
    /// configuration that needs to happen in an async method.
    async fn context_to_state(context: Arc<AppContext>) -> anyhow::Result<Self::State> {
        let state = Self::State::from(context);
        Ok(state)
    }

    fn router(_context: &AppContext) -> RouterType<Self::State>;

    fn api_docs(context: &AppContext) -> impl Fn(TransformOpenApi) -> TransformOpenApi {
        |api| {
            api.title(&context.config.app.name)
                .description(&format!("# {}", context.config.app.name))
        }
    }

    fn middleware(_context: &AppContext, _state: &Self::State) -> Vec<Box<dyn Middleware>> {
        Default::default()
    }

    fn initializers(_context: &AppContext) -> Vec<Box<dyn Initializer>> {
        Default::default()
    }

    /// Worker queue names can either be provided here, or as config values. If provided here
    /// the consumer is able to use string constants, which can be used when creating a worker
    /// instance. This can reduce the risk of copy/paste errors and typos.
    fn worker_queues(_context: &AppContext, _state: &Self::State) -> Vec<String> {
        vec![]
    }

    fn workers(_processor: &mut Processor, _context: &AppContext, _state: &Self::State) {}

    #[instrument(skip_all)]
    async fn serve<F>(
        router: Router,
        shutdown_signal: F,
        context: Arc<AppContext>,
        _state: Arc<Self::State>,
    ) -> anyhow::Result<()>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let server_addr = context.config.server.url();
        info!("Server will start at {server_addr}");

        let app_listener = tokio::net::TcpListener::bind(server_addr).await?;
        axum::serve(app_listener, router)
            .with_graceful_shutdown(shutdown_signal)
            .await?;

        Ok(())
    }

    async fn graceful_shutdown_signal(_context: Arc<AppContext>, _state: Arc<Self::State>) {
        let _output: () = future::pending().await;
    }

    #[instrument(skip_all)]
    async fn graceful_shutdown(
        _context: Arc<AppContext>,
        _state: Arc<Self::State>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

#[instrument(skip_all)]
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
) where
    F: Future<Output = ()> + Send + 'static,
{
    shutdown_signal.await;
    cancellation_token.cancel();
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
    if let Err(err) = &result {
        if context.config.app.shutdown_on_error {
            error!(
                "An error occurred in one of the app's tasks, shutting down. Error: {}",
                err
            );
            cancellation_token.cancel();
        } else {
            error!("An error occurred in one of the app's tasks: {}", err);
        }
    }
    result
}

#[instrument(skip_all)]
async fn graceful_shutdown<F1, F2>(
    shutdown_signal: F1,
    app_graceful_shutdown: F2,
    sidekiq_cancellation_token: Option<CancellationToken>,
    context: Arc<AppContext>,
) -> anyhow::Result<()>
where
    F1: Future<Output = ()> + Send + 'static,
    F2: Future<Output = anyhow::Result<()>> + Send + 'static,
{
    shutdown_signal.await;

    info!("Received shutdown signal. Shutting down gracefully.");

    info!("Closing the DB connection pool.");
    context.as_ref().clone().db.close().await?;

    if let Some(token) = sidekiq_cancellation_token {
        info!("Cancelling sidekiq workers.");
        token.cancel();
    }

    // Futures are lazy -- the custom `app_graceful_shutdown` future won't run until we call `await` on it.
    // https://rust-lang.github.io/async-book/03_async_await/01_chapter.html
    info!("Running app's custom shutdown logic.");
    app_graceful_shutdown.await?;

    Ok(())
}

#[derive(Debug)]
pub enum RouterType<S> {
    AxumRouter(Router<S>),
    AideRouter(ApiRouter<S>),
}
