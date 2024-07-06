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
use axum::extract::FromRef;
#[cfg(feature = "open-api")]
use axum::Extension;
use axum::Router;
use itertools::Itertools;
use std::collections::BTreeMap;
#[cfg(feature = "open-api")]
use std::sync::Arc;
use tracing::info;

pub struct HttpServiceBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    router: Router<S>,
    #[cfg(feature = "open-api")]
    api_router: ApiRouter<S>,
    #[cfg(feature = "open-api")]
    api_docs: Box<dyn Fn(TransformOpenApi) -> TransformOpenApi + Send>,
    middleware: BTreeMap<String, Box<dyn Middleware<S>>>,
    initializers: BTreeMap<String, Box<dyn Initializer<S>>>,
}

impl<S> HttpServiceBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub fn new(path_root: Option<&str>, state: &S) -> Self {
        // Normally, enabling a feature shouldn't remove things. In this case, however, we don't
        // want to include the default routes in the axum::Router if the `open-api` features is
        // enabled. Otherwise, we'll get a route conflict when the two routers are merged.
        #[cfg(not(feature = "open-api"))]
        let router = default_routes(path_root.unwrap_or_default(), state);
        #[cfg(feature = "open-api")]
        let router = Router::<S>::new();

        #[cfg(feature = "open-api")]
        let context = AppContext::from_ref(state);

        Self {
            state: state.clone(),
            router,
            #[cfg(feature = "open-api")]
            api_router: default_api_routes(path_root.unwrap_or_default(), state),
            #[cfg(feature = "open-api")]
            api_docs: Box::new(move |api| {
                let api = api
                    .title(&context.config().app.name)
                    .description(&format!("# {}", context.config().app.name));
                let api = if let Some(version) = context.metadata().version.as_ref() {
                    api.version(version)
                } else {
                    api
                };
                api
            }),
            middleware: default_middleware(state),
            initializers: default_initializers(state),
        }
    }

    #[cfg(test)]
    fn empty(state: &S) -> Self {
        Self {
            state: state.clone(),
            router: Router::<S>::new(),
            #[cfg(feature = "open-api")]
            api_router: ApiRouter::<S>::new(),
            #[cfg(feature = "open-api")]
            api_docs: Box::new(|op| op),
            middleware: Default::default(),
            initializers: Default::default(),
        }
    }

    pub fn router(mut self, router: Router<S>) -> Self {
        self.router = self.router.merge(router);
        self
    }

    #[cfg(feature = "open-api")]
    pub fn api_router(mut self, router: ApiRouter<S>) -> Self {
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
        T: Initializer<S> + 'static,
    {
        if !initializer.enabled(&self.state) {
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
        T: Middleware<S> + 'static,
    {
        if !middleware.enabled(&self.state) {
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
impl<A, S> AppServiceBuilder<A, S, HttpService> for HttpServiceBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn name(&self) -> String {
        NAME.to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        enabled(&AppContext::from_ref(state))
    }

    async fn build(self, state: &S) -> RoadsterResult<HttpService> {
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

        let router = router.with_state::<()>(state.clone());

        let initializers = self
            .initializers
            .values()
            .filter(|initializer| initializer.enabled(state))
            .sorted_by(|a, b| Ord::cmp(&a.priority(state), &b.priority(state)))
            .collect_vec();

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.after_router(router, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.before_middleware(router, state)
            })?;

        info!("Installing middleware. Note: the order of installation is the inverse of the order middleware will run when handling a request.");
        let router = self
            .middleware
            .values()
            .filter(|middleware| middleware.enabled(state))
            .sorted_by(|a, b| Ord::cmp(&a.priority(state), &b.priority(state)))
            // Reverse due to how Axum's `Router#layer` method adds middleware.
            .rev()
            .try_fold(router, |router, middleware| {
                info!(name=%middleware.name(), "Installing middleware");
                middleware.install(router, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.after_middleware(router, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                initializer.before_serve(router, state)
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
    use crate::service::http::initializer::MockInitializer;
    use crate::service::http::middleware::MockMiddleware;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn middleware() {
        // Arrange
        let context = AppContext::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<AppContext>::empty(&context);

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
        let context = AppContext::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<AppContext>::empty(&context);

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
        let context = AppContext::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<AppContext>::empty(&context);

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
        let context = AppContext::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<AppContext>::empty(&context);

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
        let context = AppContext::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<AppContext>::empty(&context);

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
        let context = AppContext::test(None, None, None).unwrap();
        let builder = HttpServiceBuilder::<AppContext>::empty(&context);

        let mut initializer = MockInitializer::default();
        initializer.expect_name().returning(|| "test".to_string());
        let builder = builder.initializer(initializer).unwrap();

        let mut initializer = MockInitializer::default();
        initializer.expect_name().returning(|| "test".to_string());

        // Act
        builder.initializer(initializer).unwrap();
    }
}
