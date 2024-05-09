use crate::app::App;
use crate::app_context::AppContext;
use crate::controller::default_routes;
use crate::service::http::initializer::default::default_initializers;
use crate::service::http::initializer::Initializer;
use crate::service::http::middleware::default::default_middleware;
use crate::service::http::middleware::Middleware;
use crate::service::http::service::HttpService;
use crate::service::AppServiceBuilder;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::openapi::OpenApi;
#[cfg(feature = "open-api")]
use aide::transform::TransformOpenApi;
use anyhow::bail;
use async_trait::async_trait;
#[cfg(feature = "open-api")]
use axum::Extension;
#[cfg(not(feature = "open-api"))]
use axum::Router;
use itertools::Itertools;
use std::collections::BTreeMap;
#[cfg(feature = "open-api")]
use std::sync::Arc;
use tracing::info;

pub struct HttpServiceBuilder<A: App> {
    #[cfg(not(feature = "open-api"))]
    router: Router<A::State>,
    #[cfg(feature = "open-api")]
    router: ApiRouter<A::State>,
    #[cfg(feature = "open-api")]
    api_docs: Box<dyn Fn(TransformOpenApi) -> TransformOpenApi + Send>,
    middleware: BTreeMap<String, Box<dyn Middleware<A::State>>>,
    initializers: BTreeMap<String, Box<dyn Initializer<A::State>>>,
}

impl<A: App> HttpServiceBuilder<A> {
    pub fn new(path_root: &str, context: &AppContext, state: &A::State) -> Self {
        #[cfg(feature = "open-api")]
        let app_name = context.config().app.name.clone();
        Self {
            router: default_routes(path_root, context.config()),
            #[cfg(feature = "open-api")]
            api_docs: Box::new(move |api| {
                api.title(&app_name).description(&format!("# {}", app_name))
            }),
            middleware: default_middleware(context, state),
            initializers: default_initializers(context, state),
        }
    }

    #[cfg(not(feature = "open-api"))]
    pub fn router(mut self, router: Router<A::State>) -> Self {
        self.router = self.router.merge(router);
        self
    }

    #[cfg(feature = "open-api")]
    pub fn router(mut self, router: ApiRouter<A::State>) -> Self {
        self.router = self.router.merge(router);
        self
    }

    #[cfg(feature = "open-api")]
    pub fn api_docs(
        mut self,
        api_docs: Box<dyn Fn(TransformOpenApi) -> TransformOpenApi + Send>,
    ) -> Self {
        self.api_docs = api_docs;
        self
    }

    pub fn initializer(
        mut self,
        initializer: Box<dyn Initializer<A::State>>,
    ) -> anyhow::Result<Self> {
        let name = initializer.name();
        if self
            .initializers
            .insert(name.clone(), initializer)
            .is_some()
        {
            bail!("Initializer `{name}` was already registered");
        }
        Ok(self)
    }

    pub fn middleware(mut self, middleware: Box<dyn Middleware<A::State>>) -> anyhow::Result<Self> {
        let name = middleware.name();
        if self.middleware.insert(name.clone(), middleware).is_some() {
            bail!("Middleware `{name}` was already registered");
        }
        Ok(self)
    }
}

#[async_trait]
impl<A: App> AppServiceBuilder<A, HttpService> for HttpServiceBuilder<A> {
    async fn build(self, context: &AppContext, state: &A::State) -> anyhow::Result<HttpService> {
        #[cfg(not(feature = "open-api"))]
        let router = self.router;

        #[cfg(feature = "open-api")]
        let (router, api) = {
            let mut api = OpenApi::default();
            let api_docs = self.api_docs;
            let router = self.router.finish_api_with(&mut api, api_docs);
            // Arc is very important here or we will face massive memory and performance issues
            let api = Arc::new(api);
            let router = router.layer(Extension(api.clone()));
            (router, api)
        };

        let router = router.with_state::<()>(state.clone());

        let initializers = self
            .initializers
            .values()
            .filter(|initializer| initializer.enabled(context, state))
            .sorted_by(|a, b| Ord::cmp(&a.priority(context, state), &b.priority(context, state)))
            .collect_vec();

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.after_router(router, context, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.before_middleware(router, context, state)
            })?;

        info!("Installing middleware. Note: the order of installation is the inverse of the order middleware will run when handling a request.");
        let router = self
            .middleware
            .values()
            .filter(|middleware| middleware.enabled(context, state))
            .sorted_by(|a, b| Ord::cmp(&a.priority(context, state), &b.priority(context, state)))
            // Reverse due to how Axum's `Router#layer` method adds middleware.
            .rev()
            .try_fold(router, |router, middleware| {
                info!(middleware=%middleware.name(), "Installing middleware");
                middleware.install(router, context, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.after_middleware(router, context, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.before_serve(router, context, state)
            })?;

        let service = HttpService {
            router,
            #[cfg(feature = "open-api")]
            api,
        };

        Ok(service)
    }
}
