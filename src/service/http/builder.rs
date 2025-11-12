#[cfg(feature = "open-api")]
use crate::api::http::default_api_routes;
#[cfg(not(feature = "open-api"))]
use crate::api::http::default_routes;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::ServiceBuilder;
use crate::service::http::initializer::Initializer;
use crate::service::http::initializer::default::default_initializers;
use crate::service::http::middleware::Middleware;
use crate::service::http::middleware::default::default_middleware;
use crate::service::http::service::{HttpService, NAME, enabled};
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::openapi::OpenApi;
#[cfg(feature = "open-api")]
use aide::transform::TransformOpenApi;
use async_trait::async_trait;
#[cfg(feature = "open-api")]
use axum::Extension;
use axum::Router;
use axum_core::extract::FromRef;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::sync::Arc;
use tracing::info;

#[cfg(feature = "open-api")]
type ApiDocs = Box<dyn Send + Fn(TransformOpenApi) -> TransformOpenApi>;

pub struct HttpServiceBuilder<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    state: S,
    router: Router<S>,
    #[cfg(feature = "open-api")]
    api_router: ApiRouter<S>,
    #[cfg(feature = "open-api")]
    api_docs: ApiDocs,
    middleware: BTreeMap<String, Box<dyn Middleware<S, Error = crate::error::Error>>>,
    initializers: BTreeMap<String, Box<dyn Initializer<S, Error = crate::error::Error>>>,
}

impl<S> HttpServiceBuilder<S>
where
    S: 'static + Send + Sync + Clone,
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
                if let Some(version) = context.metadata().version.as_ref() {
                    api.version(version)
                } else {
                    api
                }
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
        self.api_router = self.api_router.merge(router);
        self
    }

    #[cfg(feature = "open-api")]
    pub fn api_docs(mut self, api_docs: ApiDocs) -> Self {
        self.api_docs = api_docs;
        self
    }

    pub fn initializer<T>(mut self, initializer: T) -> RoadsterResult<Self>
    where
        T: 'static + Send + Sync + Initializer<S>,
    {
        if !initializer.enabled(&self.state) {
            return Ok(self);
        }
        let name = initializer.name();
        if self
            .initializers
            .insert(name.clone(), Box::new(InitializerWrapper::new(initializer)))
            .is_some()
        {
            return Err(crate::error::other::OtherError::Message(format!(
                "Initializer `{name}` was already registered"
            ))
            .into());
        }
        Ok(self)
    }

    pub fn middleware<T>(mut self, middleware: T) -> RoadsterResult<Self>
    where
        T: 'static + Send + Sync + Middleware<S>,
    {
        if !middleware.enabled(&self.state) {
            return Ok(self);
        }
        let name = middleware.name();
        if self
            .middleware
            .insert(name.clone(), Box::new(MiddlewareWrapper::new(middleware)))
            .is_some()
        {
            return Err(crate::error::other::OtherError::Message(format!(
                "Middleware `{name}` was already registered"
            ))
            .into());
        }
        Ok(self)
    }
}

#[async_trait]
impl<S> ServiceBuilder<S, HttpService> for HttpServiceBuilder<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

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
                info!(http_initializer.name=%initializer.name(), "Running Initializer::after_router");
                initializer.after_router(router, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                info!(http_initializer.name=%initializer.name(), "Running Initializer::before_middleware");
                initializer.before_middleware(router, state)
            })?;

        info!(
            "Installing middleware. Note: the order of installation is the inverse of the order middleware will run when handling a request."
        );
        let router = self
            .middleware
            .values()
            .filter(|middleware| middleware.enabled(state))
            .sorted_by(|a, b| Ord::cmp(&a.priority(state), &b.priority(state)))
            // Reverse due to how Axum's `Router#layer` method adds middleware.
            .rev()
            .try_fold(router, |router, middleware| {
                info!(http_middleware.name=%middleware.name(), "Installing middleware");
                middleware.install(router, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                info!(http_initializer.name=%initializer.name(), "Running Initializer::after_middleware");
                initializer.after_middleware(router, state)
            })?;

        let router = initializers
            .iter()
            .try_fold(router, |router, initializer| {
                info!(http_initializer.name=%initializer.name(), "Running Initializer::before_serve");
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

type EnabledFn<S> = Box<dyn Send + Sync + for<'a> Fn(&'a S) -> bool>;
type PriorityFn<S> = Box<dyn Send + Sync + for<'a> Fn(&'a S) -> i32>;

type RouterAndStateFn<S> =
    Box<dyn Send + Sync + for<'a> Fn(Router, &'a S) -> RoadsterResult<Router>>;

pub(crate) struct InitializerWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    name: String,
    enabled_fn: EnabledFn<S>,
    priority_fn: PriorityFn<S>,
    after_router_fn: RouterAndStateFn<S>,
    before_middleware_fn: RouterAndStateFn<S>,
    after_middleware_fn: RouterAndStateFn<S>,
    before_serve_fn: RouterAndStateFn<S>,
}

impl<S> InitializerWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) fn new<T>(initializer: T) -> Self
    where
        T: 'static + Send + Sync + Initializer<S>,
    {
        let name = initializer.name();
        let initializer = Arc::new(initializer);
        let enabled_fn: EnabledFn<S> = {
            let initializer = initializer.clone();
            Box::new(move |state| initializer.enabled(state))
        };
        let priority_fn: PriorityFn<S> = {
            let initializer = initializer.clone();
            Box::new(move |state| initializer.priority(state))
        };
        let after_router_fn: RouterAndStateFn<S> = {
            let initializer = initializer.clone();
            Box::new(move |router, state| {
                let router = initializer
                    .after_router(router, state)
                    .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                Ok(router)
            })
        };
        let before_middleware_fn: RouterAndStateFn<S> = {
            let initializer = initializer.clone();
            Box::new(move |router, state| {
                let router = initializer
                    .before_middleware(router, state)
                    .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                Ok(router)
            })
        };
        let after_middleware_fn: RouterAndStateFn<S> = {
            let initializer = initializer.clone();
            Box::new(move |router, state| {
                let router = initializer
                    .after_middleware(router, state)
                    .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                Ok(router)
            })
        };
        let before_serve_fn: RouterAndStateFn<S> = {
            let initializer = initializer.clone();
            Box::new(move |router, state| {
                let router = initializer
                    .before_serve(router, state)
                    .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                Ok(router)
            })
        };
        Self {
            name,
            enabled_fn,
            priority_fn,
            after_router_fn,
            before_middleware_fn,
            after_middleware_fn,
            before_serve_fn,
        }
    }
}

#[async_trait]
impl<S> Initializer<S> for InitializerWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        (self.enabled_fn)(state)
    }

    fn priority(&self, state: &S) -> i32 {
        (self.priority_fn)(state)
    }

    fn after_router(&self, router: Router, state: &S) -> Result<Router, Self::Error> {
        (self.after_router_fn)(router, state)
    }

    fn before_middleware(&self, router: Router, state: &S) -> Result<Router, Self::Error> {
        (self.before_middleware_fn)(router, state)
    }

    fn after_middleware(&self, router: Router, state: &S) -> Result<Router, Self::Error> {
        (self.after_middleware_fn)(router, state)
    }

    fn before_serve(&self, router: Router, state: &S) -> Result<Router, Self::Error> {
        (self.before_serve_fn)(router, state)
    }
}

pub(crate) struct MiddlewareWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    name: String,
    enabled_fn: EnabledFn<S>,
    priority_fn: PriorityFn<S>,
    install_fn: RouterAndStateFn<S>,
}

impl<S> MiddlewareWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) fn new<T>(middleware: T) -> Self
    where
        T: 'static + Send + Sync + Middleware<S>,
    {
        let name = middleware.name();
        let middleware = Arc::new(middleware);
        let enabled_fn: EnabledFn<S> = {
            let middleware = middleware.clone();
            Box::new(move |state| middleware.enabled(state))
        };
        let priority_fn: PriorityFn<S> = {
            let middleware = middleware.clone();
            Box::new(move |state| middleware.priority(state))
        };
        let install_fn: RouterAndStateFn<S> = {
            let middleware = middleware.clone();
            Box::new(move |router, state| {
                let router = middleware
                    .install(router, state)
                    .map_err(|err| crate::error::other::OtherError::Other(Box::new(err)))?;
                Ok(router)
            })
        };
        Self {
            name,
            enabled_fn,
            priority_fn,
            install_fn,
        }
    }
}

#[async_trait]
impl<S> Middleware<S> for MiddlewareWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        (self.enabled_fn)(state)
    }

    fn priority(&self, state: &S) -> i32 {
        (self.priority_fn)(state)
    }

    fn install(&self, router: Router, state: &S) -> Result<Router, Self::Error> {
        (self.install_fn)(router, state)
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
