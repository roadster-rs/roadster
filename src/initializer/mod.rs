pub mod default;
pub mod normalize_path;

use crate::app_context::AppContext;
use axum::Router;

pub trait Initializer {
    fn name(&self) -> String;

    fn enabled(&self, context: &AppContext) -> bool;

    fn priority(&self, context: &AppContext) -> i32;

    fn after_router(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn before_middleware(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn after_middleware(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        Ok(router)
    }

    fn before_serve(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        Ok(router)
    }
}
