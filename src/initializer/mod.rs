pub mod default;
pub mod normalize_path;

use crate::app_context::AppContext;
use axum::Router;

/// Provides hooks into various stages of the app's startup to allow initializing and installing
/// anything that needs to be done during a specific stage of startup. The type `S` is the
/// custom [crate::app::App::State] defined for the app.
pub trait Initializer<S> {
    fn name(&self) -> String;

    fn enabled(&self, context: &AppContext, state: &S) -> bool;

    fn priority(&self, context: &AppContext, state: &S) -> i32;

    fn after_router(
        &self,
        router: Router,
        _context: &AppContext,
        _state: &S,
    ) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn before_middleware(
        &self,
        router: Router,
        _context: &AppContext,
        _state: &S,
    ) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn after_middleware(
        &self,
        router: Router,
        _context: &AppContext,
        _state: &S,
    ) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn before_serve(
        &self,
        router: Router,
        _context: &AppContext,
        _state: &S,
    ) -> anyhow::Result<Router> {
        Ok(router)
    }
}
