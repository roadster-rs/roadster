use crate::app_context::AppContext;
use crate::controller::middleware::Middleware;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use tower_http::catch_panic::CatchPanicLayer;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CatchPanicConfig {}

pub struct CatchPanicMiddleware;
impl Middleware for CatchPanicMiddleware {
    fn name(&self) -> String {
        "catch-panic".to_string()
    }

    fn enabled(&self, context: &AppContext) -> bool {
        context
            .config
            .middleware
            .catch_panic
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext) -> i32 {
        context.config.middleware.catch_panic.common.priority
    }

    fn install(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        let router = router.layer(CatchPanicLayer::new());

        Ok(router)
    }
}
