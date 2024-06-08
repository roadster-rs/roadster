#[cfg(feature = "open-api")]
use crate::api::http::default_api_routes;
#[cfg(not(feature = "open-api"))]
use crate::api::http::default_routes;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::service::http::initializer::default::default_initializers;
use crate::service::http::initializer::Initializer;
use crate::service::http::middleware::default::default_middleware;
use crate::service::http::middleware::Middleware;
use crate::service::http::service::{enabled, HttpService, NAME};
use crate::service::AppServiceBuilder;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::openapi::OpenApi;
#[cfg(feature = "open-api")]
use aide::transform::TransformOpenApi;
use anyhow::anyhow;
use async_trait::async_trait;
#[cfg(feature = "open-api")]
use axum::Extension;
use axum::Router;
use itertools::Itertools;
use std::collections::BTreeMap;
#[cfg(feature = "open-api")]
use std::sync::Arc;
use tracing::info;

pub struct HttpServiceBuilder<A: App + 'static> {
    context: AppContext<A::State>,
    router: Router<AppContext<A::State>>,
    #[cfg(feature = "open-api")]
    api_router: ApiRouter<AppContext<A::State>>,
    #[cfg(feature = "open-api")]
    api_docs: Box<dyn Fn(TransformOpenApi) -> TransformOpenApi + Send>,
    middleware: BTreeMap<String, Box<dyn Middleware<A::State>>>,
    initializers: BTreeMap<String, Box<dyn Initializer<A::State>>>,
}

impl<A: App> HttpServiceBuilder<A> {
    pub fn new(path_root: Option<&str>, context: &AppContext<A::State>) -> Self {
        // Normally, enabling a feature shouldn't remove things. In this case, however, we don't
        // want to include the default routes in the axum::Router if the `open-api` features is
        // enabled. Otherwise, we'll get a route conflict when the two routers are merged.
        #[cfg(not(feature = "open-api"))]
        let router = default_routes(path_root.unwrap_or_default(), context);
        #[cfg(feature = "open-api")]
        let router = Router::<AppContext<A::State>>::new();

        #[cfg(feature = "open-api")]
        let app_name = context.config().app.name.clone();
        Self {
            context: context.clone(),
            router,
            #[cfg(feature = "open-api")]
            api_router: default_api_routes(path_root.unwrap_or_default(), context),
            #[cfg(feature = "open-api")]
            api_docs: Box::new(move |api| {
                api.title(&app_name).description(&format!("# {}", app_name))
            }),
            middleware: default_middleware(context),
            initializers: default_initializers(context),
        }
    }

    #[cfg(test)]
    fn empty(context: &AppContext<A::State>) -> Self {
        Self {
            context: context.clone(),
            router: Router::<AppContext<A::State>>::new(),
            #[cfg(feature = "open-api")]
            api_router: ApiRouter::<AppContext<A::State>>::new(),
            #[cfg(feature = "open-api")]
            api_docs: Box::new(|op| op),
            middleware: Default::default(),
            initializers: Default::default(),
        }
    }

    pub fn router(mut self, router: Router<AppContext<A::State>>) -> Self {
        self.router = self.router.merge(router);
        self
    }

    #[cfg(feature = "open-api")]
    pub fn api_router(mut self, router: ApiRouter<AppContext<A::State>>) -> Self {
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

    pub fn initializer<T>(mut self, initializer: T) -> RoadsterResult<Self>
    where
        T: Initializer<A::State> + 'static,
    {
        if !initializer.enabled(&self.context) {
            return Ok(self);
        }
        let name = initializer.name();
        if self
            .initializers
            .insert(name.clone(), Box::new(initializer))
            .is_some()
        {
            return Err(anyhow!("Initializer `{name}` was already registered").into());
        }
        Ok(self)
    }

    pub fn middleware<T>(mut self, middleware: T) -> RoadsterResult<Self>
    where
        T: Middleware<A::State> + 'static,
    {
        if !middleware.enabled(&self.context) {
            return Ok(self);
        }
        let name = middleware.name();
        if self
            .middleware
            .insert(name.clone(), Box::new(middleware))
            .is_some()
        {
            return Err(anyhow!("Middleware `{name}` was already registered").into());
        }
        Ok(self)
    }
}

#[async_trait]
impl<A: App> AppServiceBuilder<A, HttpService> for HttpServiceBuilder<A> {
    fn name(&self) -> String {
        NAME.to_string()
    }

    fn enabled(&self, app_context: &AppContext<A::State>) -> bool {
        enabled(app_context)
    }

    async fn build(self, context: &AppContext<A::State>) -> RoadsterResult<HttpService> {
        let router = self.router;

        #[cfg(feature = "open-api")]
        let (router, api) = {
            let mut api = OpenApi::default();
            let api_docs = self.api_docs;
            let api_router = self.api_router.finish_api_with(&mut api, api_docs);
            let router = router.merge(api_router);
            // Arc is very important here or we will face massive memory and performance issues
            let api = Arc::new(api);
            let router = router.layer(Extension(api.clone()));
            (router, api)
        };

        let router = router.with_state::<()>(context.clone());

        let initializers = self
            .initializers
            .values()
            .filter(|initializer| initializer.enabled(context))
            .sorted_by(|a, b| Ord::cmp(&a.priority(context), &b.priority(context)))
            .collect_vec();

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.after_router(router, context)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.before_middleware(router, context)
            })?;

        info!("Installing middleware. Note: the order of installation is the inverse of the order middleware will run when handling a request.");
        let router = self
            .middleware
            .values()
            .filter(|middleware| middleware.enabled(context))
            .sorted_by(|a, b| Ord::cmp(&a.priority(context), &b.priority(context)))
            // Reverse due to how Axum's `Router#layer` method adds middleware.
            .rev()
            .try_fold(router, |router, middleware| {
                info!(middleware=%middleware.name(), "Installing middleware");
                middleware.install(router, context)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.after_middleware(router, context)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.before_serve(router, context)
            })?;

        let service = HttpService {
            router,
            #[cfg(feature = "open-api")]
            api,
        };

        Ok(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::context::AppContext;
    use crate::app::MockApp;
    use crate::service::http::initializer::MockInitializer;
    use crate::service::http::middleware::MockMiddleware;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn middleware() {
        // Arrange
        let context = AppContext::<()>::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<MockApp>::empty(&context);

        let mut middleware = MockMiddleware::default();
        middleware.expect_enabled().returning(|_| true);
        middleware.expect_name().returning(|| "test".to_string());

        // Act
        let builder = builder.middleware(middleware).unwrap();

        // Assert
        assert_eq!(builder.middleware.len(), 1);
        assert!(builder.middleware.contains_key("test"));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn middleware_not_enabled() {
        // Arrange
        let context = AppContext::<()>::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<MockApp>::empty(&context);

        let mut middleware = MockMiddleware::default();
        middleware.expect_enabled().returning(|_| false);

        // Act
        let builder = builder.middleware(middleware).unwrap();

        // Assert
        assert!(builder.middleware.is_empty());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[should_panic]
    fn middleware_already_registered() {
        // Arrange
        let context = AppContext::<()>::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<MockApp>::empty(&context);

        let mut middleware = MockMiddleware::default();
        middleware.expect_name().returning(|| "test".to_string());
        let builder = builder.middleware(middleware).unwrap();

        let mut middleware = MockMiddleware::default();
        middleware.expect_name().returning(|| "test".to_string());

        // Act
        builder.middleware(middleware).unwrap();
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn initializer() {
        // Arrange
        let context = AppContext::<()>::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<MockApp>::empty(&context);

        let mut initializer = MockInitializer::default();
        initializer.expect_enabled().returning(|_| true);
        initializer.expect_name().returning(|| "test".to_string());

        // Act
        let builder = builder.initializer(initializer).unwrap();

        // Assert
        assert_eq!(builder.initializers.len(), 1);
        assert!(builder.initializers.contains_key("test"));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn initializer_not_enabled() {
        // Arrange
        let context = AppContext::<()>::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<MockApp>::empty(&context);

        let mut initializer = MockInitializer::default();
        initializer.expect_enabled().returning(|_| false);

        // Act
        let builder = builder.initializer(initializer).unwrap();

        // Assert
        assert!(builder.initializers.is_empty());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[should_panic]
    fn initializer_already_registered() {
        // Arrange
        let context = AppContext::<()>::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<MockApp>::empty(&context);

        let mut initializer = MockInitializer::default();
        initializer.expect_name().returning(|| "test".to_string());
        let builder = builder.initializer(initializer).unwrap();

        let mut initializer = MockInitializer::default();
        initializer.expect_name().returning(|| "test".to_string());

        // Act
        builder.initializer(initializer).unwrap();
    }
}
