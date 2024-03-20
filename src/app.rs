use aide::axum::ApiRouter;
use aide::openapi::OpenApi;
use aide::transform::TransformOpenApi;
use async_trait::async_trait;

use axum::{Extension, Router};

use itertools::Itertools;
use std::sync::Arc;
use tracing::{debug, info, instrument};

use crate::app_context::AppContext;
use crate::config::app_config::AppConfig;
use crate::controller::middleware::default::default_middleware;
use crate::controller::middleware::Middleware;
use crate::tracing::init_tracing;

pub async fn start<A>() -> anyhow::Result<()>
where
    A: App + Default + Send + Sync,
{
    let config = AppConfig::new()?;

    A::init_tracing(&config)?;

    debug!("{config:?}");

    let mut context = AppContext::new(config).await?;

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
    let router = router.with_state::<()>(state);

    // Install middleware, both the default middleware and any provided by the consumer.
    let router = default_middleware()
        .into_iter()
        .chain(A::middleware(&context).into_iter())
        .unique_by(|middleware| middleware.name())
        .filter(|middleware| middleware.enabled(&context))
        .sorted_by(|a, b| Ord::cmp(&a.priority(&context), &b.priority(&context)))
        // Reverse due to how Axum's `Router#layer` method adds middleware.
        .rev()
        .try_fold(router, |router, middleware| {
            middleware.install(router, &context)
        })?;

    A::serve(&context, router).await?;

    Ok(())
}

#[async_trait]
pub trait App {
    type State: From<Arc<AppContext>> + Into<Arc<AppContext>> + Clone + Send + Sync + 'static;

    fn init_tracing(config: &AppConfig) -> anyhow::Result<()> {
        init_tracing(config)?;

        Ok(())
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

    fn middleware(_context: &AppContext) -> Vec<Box<dyn Middleware>> {
        Default::default()
    }

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
