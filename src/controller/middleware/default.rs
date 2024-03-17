use crate::app_context::AppContext;
use crate::controller::middleware::bulk::BulkMiddleware;
use crate::controller::middleware::request_id::{
    PropagateRequestIdMiddleware, SetRequestIdMiddleware,
};
use crate::controller::middleware::sensitive_headers::{
    SensitiveRequestHeadersMiddleware, SensitiveResponseHeadersMiddleware,
};
use crate::controller::middleware::tracing::TracingMiddleware;
use crate::controller::middleware::Middleware;
use axum::Router;

pub struct DefaultMiddleware;
impl Middleware for DefaultMiddleware {
    fn name(&self) -> String {
        "roadster-defaults".to_string()
    }

    fn install(&self, router: Router, context: &AppContext) -> anyhow::Result<Router> {
        let middleware: Vec<Box<dyn Middleware>> = vec![
            Box::new(SensitiveRequestHeadersMiddleware),
            Box::new(SetRequestIdMiddleware),
            Box::new(TracingMiddleware),
            Box::new(PropagateRequestIdMiddleware),
            Box::new(SensitiveResponseHeadersMiddleware),
        ];

        BulkMiddleware::default()
            .append_all(middleware)
            .install(router, context)
    }
}
