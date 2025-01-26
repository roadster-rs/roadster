use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::Router;
use axum_core::extract::FromRef;
use typed_builder::TypedBuilder;

type ApplyFn<S> = Box<dyn Fn(Router, &S) -> RoadsterResult<Router> + Send>;

/// A [`Middleware`] that can be applied without creating a separate `struct`. Useful to easily
/// apply a middleware that's based on a function, for example.
///
/// # Examples
/// ```rust
/// # use axum::response::Response;
/// # use axum::middleware::Next;
/// # use axum_core::extract::Request;
/// # use tracing::info;
/// # use roadster::service::http::middleware::any::AnyMiddleware;
/// #
/// pub(crate) async fn hello_world_middleware_fn(request: Request, next: Next) -> Response {
///     info!("Running `hello-world` middleware");
///
///     next.run(request).await
/// }
///
/// let middleware = AnyMiddleware::builder()
///     .name("hello-world")
///     .enabled(true)
///     .apply(|router, _state| {
///         Ok(router
///             .layer(axum::middleware::from_fn(hello_world_middleware_fn)))
///     })
///     .build();
/// ```
#[derive(TypedBuilder)]
#[non_exhaustive]
pub struct AnyMiddleware<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[builder(setter(into))]
    name: String,
    #[builder(default, setter(strip_option))]
    enabled: Option<bool>,
    #[builder(default, setter(strip_option))]
    priority: Option<i32>,
    #[builder(setter(transform = |a: impl Fn(Router, &S) -> RoadsterResult<Router> + Send + 'static| to_box_fn(a) ))]
    apply: ApplyFn<S>,
}

fn to_box_fn<S>(p: impl Fn(Router, &S) -> RoadsterResult<Router> + Send + 'static) -> ApplyFn<S> {
    Box::new(p)
}

impl<S> Middleware<S> for AnyMiddleware<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
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
        (self.apply)(router, state)
    }
}
