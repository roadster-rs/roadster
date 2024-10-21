pub mod any;
pub mod default;
pub mod normalize_path;

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use axum::Router;
use axum_core::extract::FromRef;

/// Provides hooks into various stages of the app's startup to allow initializing and installing
/// anything that needs to be done during a specific stage of startup of the HTTP service.
#[cfg_attr(test, mockall::automock)]
pub trait Initializer<S>: Send
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String;

    fn enabled(&self, state: &S) -> bool;

    /// Used to determine the order in which the initializer will run during app initialization.
    /// Smaller numbers will run before larger numbers. For example, an initializer with priority
    /// `-10` will run before an initializer with priority `10`.
    ///
    /// If two initializers have the same priority, they are not guaranteed to run in any particular
    /// order relative to each other. This may be fine for many initializers.
    ///
    /// If the order in which your initializer runs doesn't particularly matter, it's generally
    /// safe to set its priority as `0`.
    fn priority(&self, state: &S) -> i32;

    fn after_router(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        Ok(router)
    }

    fn before_middleware(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        Ok(router)
    }

    fn after_middleware(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        Ok(router)
    }

    fn before_serve(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        Ok(router)
    }
}
