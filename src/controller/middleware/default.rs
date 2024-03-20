use crate::controller::middleware::request_id::{
    PropagateRequestIdMiddleware, SetRequestIdMiddleware,
};
use crate::controller::middleware::sensitive_headers::{
    SensitiveRequestHeadersMiddleware, SensitiveResponseHeadersMiddleware,
};
use crate::controller::middleware::tracing::TracingMiddleware;
use crate::controller::middleware::Middleware;

pub fn default_middleware() -> Vec<Box<dyn Middleware>> {
    vec![
        Box::new(SensitiveRequestHeadersMiddleware),
        Box::new(SensitiveResponseHeadersMiddleware),
        Box::new(SetRequestIdMiddleware),
        Box::new(PropagateRequestIdMiddleware),
        Box::new(TracingMiddleware),
    ]
}
