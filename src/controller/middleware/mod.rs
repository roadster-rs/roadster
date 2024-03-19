pub mod default;
pub mod request_id;
pub mod sensitive_headers;
pub mod tracing;

use crate::app_context::AppContext;
use axum::Router;

// Todo: add a `priority` method to enable more control over the order in which the middleware runs?
//  Also, make the priority configurable?
//  How does this affect our ability to provide defaults? How to set the priority of a new
//  middleware?
pub trait Middleware {
    fn name(&self) -> String;
    fn enabled(&self, context: &AppContext) -> bool;
    fn priority(&self, context: &AppContext) -> i32;
    fn install(&self, router: Router, context: &AppContext) -> anyhow::Result<Router>;
}
