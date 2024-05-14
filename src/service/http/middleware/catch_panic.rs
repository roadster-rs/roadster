#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use tower_http::catch_panic::CatchPanicLayer;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CatchPanicConfig {}

pub struct CatchPanicMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for CatchPanicMiddleware {
    fn name(&self) -> String {
        "catch-panic".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .catch_panic
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext<S>) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .catch_panic
            .common
            .priority
    }

    fn install(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        let router = router.layer(CatchPanicLayer::new());

        Ok(router)
    }
}
