pub mod default;
pub mod request_id;
pub mod sensitive_headers;
pub mod tracing;

use crate::app_context::AppContext;
use axum::Router;

pub trait Middleware {
    fn name(&self) -> String;
    fn enabled(&self, context: &AppContext) -> bool;
    fn priority(&self, context: &AppContext) -> i32;
    fn install(&self, router: Router, context: &AppContext) -> anyhow::Result<Router>;
}
