use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::routing::Route;
use axum::Router;
use axum_core::extract::{FromRef, Request};
use axum_core::response::IntoResponse;
use std::convert::Infallible;
use tower::{Layer, Service};
use typed_builder::TypedBuilder;

#[derive(TypedBuilder)]
pub struct AnyMiddleware<S, L>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    // Layer constrains copied from https://docs.rs/axum/0.7.7/axum/routing/struct.Router.html#method.layer
    L: Layer<Route> + Clone + Send + 'static,
    L::Service: Service<Request> + Clone + Send + 'static,
    <L::Service as Service<Request>>::Response: IntoResponse + 'static,
    <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
    <L::Service as Service<Request>>::Future: Send + 'static,
{
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(strip_option))]
    enabled: Option<bool>,
    #[builder(default, setter(strip_option))]
    priority: Option<i32>,
    #[builder(setter(transform = |p: impl Fn(&S) -> L + Send + 'static| to_box_fn(p) ))]
    layer_provider: Box<dyn Fn(&S) -> L + Send>,
}

fn to_box_fn<S, L>(p: impl Fn(&S) -> L + Send + 'static) -> Box<dyn Fn(&S) -> L + Send> {
    Box::new(p)
}

impl<S, L> Middleware<S> for AnyMiddleware<S, L>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    // Layer constrains copied from https://docs.rs/axum/0.7.7/axum/routing/struct.Router.html#method.layer
    L: Layer<Route> + Clone + Send + 'static,
    L::Service: Service<Request> + Clone + Send + 'static,
    <L::Service as Service<Request>>::Response: IntoResponse + 'static,
    <L::Service as Service<Request>>::Error: Into<Infallible> + 'static,
    <L::Service as Service<Request>>::Future: Send + 'static,
{
    fn name(&self) -> String {
        self.name.clone()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        let config = context
            .config()
            .service
            .http
            .custom
            .middleware
            .custom
            .get(&self.name);
        if let Some(config) = config {
            config.common.enabled(state)
        } else {
            context
                .config()
                .service
                .http
                .custom
                .middleware
                .default_enable
                || self.enabled.unwrap_or_default()
        }
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .custom
            .get(&self.name)
            .map(|config| config.common.priority)
            .unwrap_or_else(|| self.priority.unwrap_or_default())
    }

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let router = router.layer((self.layer_provider)(state));

        Ok(router)
    }
}
