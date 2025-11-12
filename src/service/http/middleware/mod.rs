pub mod any;
pub mod cache_control;
pub mod catch_panic;
pub mod compression;
pub mod cors;
pub mod default;
pub mod etag;
pub mod request_id;
pub mod sensitive_headers;
pub mod size_limit;
pub mod timeout;
pub mod tracing;

use crate::app::context::AppContext;
use axum::Router;
use axum_core::extract::FromRef;

/// Allows initializing and installing middleware on the app's [Router].
///
/// This trait is provided in addition to [crate::service::http::initializer::Initializer] because installing
/// middleware is a bit of a special case compared to a general initializer:
///     1. The order in which middleware runs matters. For example, we want
///        [tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer] to run before
///        [tower_http::trace::TraceLayer] to avoid logging sensitive headers.
///     2. Because of how axum's [Router::layer] method installs middleware, the order in which
///        middleware is installed is the reverse of the order it will run when handling a request.
///        Therefore, we install the middleware in the reverse order that we want it to run (this
///        is done automatically by Roadster based on [Middleware::priority]).
#[cfg_attr(test, mockall::automock(type Error = crate::error::Error;))]
pub trait Middleware<S>: Send
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error: Send + Sync + std::error::Error;

    fn name(&self) -> String;
    fn enabled(&self, state: &S) -> bool;
    /// Used to determine the order in which the middleware will run when handling a request. Smaller
    /// numbers will run before larger numbers. For example, a middleware with priority `-10`
    /// will run before a middleware with priority `10`.
    ///
    /// If two middlewares have the same priority, they are not guaranteed to run or be installed
    /// in any particular order relative to each other. This may be fine for many middlewares.
    ///
    /// If the order in which your middleware runs doesn't particularly matter, it's generally
    /// safe to set its priority as `0`.
    ///
    /// Note: Because of how axum's [Router::layer] method installs middleware, the order in which
    /// middleware is installed is the reverse of the order it will run when handling a request.
    /// Therefore, we install the middleware in the reverse order that we want it to run (this
    /// is done automatically by Roadster based on [Middleware::priority]). So, a middleware
    /// with priority `-10` will be _installed after_ a middleware with priority `10`, which will
    /// allow the middleware with priority `-10` to _run before_ a middleware with priority `10`.
    fn priority(&self, state: &S) -> i32;
    fn install(&self, router: Router, state: &S) -> Result<Router, Self::Error>;
}
