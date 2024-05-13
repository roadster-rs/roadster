pub mod default;
pub mod normalize_path;

#[mockall_double::double]
use crate::app_context::AppContext;
use axum::Router;

/// Provides hooks into various stages of the app's startup to allow initializing and installing
/// anything that needs to be done during a specific stage of startup. The type `S` is the
/// custom [crate::app::App::State] defined for the app.
pub trait Initializer<S>: Send {
    fn name(&self) -> String;

    fn enabled(&self, context: &AppContext<S>) -> bool;

    /// Used to determine the order in which the initializer will run during app initialization.
    /// Smaller numbers will run before larger numbers. For example, an initializer with priority
    /// `-10` will run before an initializer with priority `10`.
    ///
    /// If two initializers have the same priority, they are not guaranteed to run in any particular
    /// order relative to each other. This may be fine for many initializers.
    ///
    /// If the order in which your initializer runs doesn't particularly matter, it's generally
    /// safe to set its priority as `0`.
    fn priority(&self, context: &AppContext<S>) -> i32;

    fn after_router(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn before_middleware(
        &self,
        router: Router,
        _context: &AppContext<S>,
    ) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn after_middleware(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn before_serve(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        Ok(router)
    }
}
