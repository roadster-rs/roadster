pub(crate) mod bulk;
pub mod default;
pub mod sensitive_headers;

use crate::app_context::AppContext;
use axum::Router;

pub trait Middleware {
    fn name(&self) -> String;
    fn install(&self, router: Router, context: &AppContext) -> Router;
}
