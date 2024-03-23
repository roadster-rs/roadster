use std::sync::Arc;
use std::time::Duration;

use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use aide::transform::TransformOpenApi;
use async_trait::async_trait;
use axum::{Extension, Router};
use itertools::Itertools;
use sea_orm::{ConnectOptions, Database};
use sea_orm_migration::MigratorTrait;
use sidekiq::{Processor, Worker};
use tracing::{debug, info, instrument};

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
    A: App + Default + Send + Sync,
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

    let redis = config
        .worker
        .as_ref()
        .and_then(|worker| worker.redis.as_ref());
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
    let router = default_middleware()
        .into_iter()
        .chain(A::middleware(&context, &state).into_iter())
        .filter(|middleware| middleware.enabled(&context))
        .unique_by(|middleware| middleware.name())
        .sorted_by(|a, b| Ord::cmp(&a.priority(&context), &b.priority(&context)))
        // Reverse due to how Axum's `Router#layer` method adds middleware.
        .rev()
        .try_fold(router, |router, middleware| {
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

    let mut processor = redis.map(|redis| {
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
        Processor::new(redis, queue_names)
    });
    if let Some(processor) = &mut processor {
        A::workers(processor, &context, &state);
    }
    let processor = || {
        Box::pin(async {
            if let Some(processor) = processor {
                processor.run().await;
            }
            Ok(())
        })
    };

    tokio::try_join!(A::serve(&context, router), processor())?;

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
    async fn serve(context: &AppContext, router: Router) -> anyhow::Result<()> {
        let server_addr = context.config.server.url();
        info!("Server will start at {server_addr}");

        let app_listener = tokio::net::TcpListener::bind(server_addr).await?;
        axum::serve(app_listener, router).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum RouterType<S> {
    AxumRouter(Router<S>),
    AideRouter(ApiRouter<S>),
}
