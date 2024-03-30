use crate::controller::middleware::catch_panic::CatchPanicMiddleware;
use crate::controller::middleware::compression::RequestDecompressionMiddleware;
use crate::controller::middleware::request_id::{
    PropagateRequestIdMiddleware, SetRequestIdMiddleware,
};
use crate::controller::middleware::sensitive_headers::{
    SensitiveRequestHeadersMiddleware, SensitiveResponseHeadersMiddleware,
};
use crate::controller::middleware::size_limit::RequestBodyLimitMiddleware;
use crate::controller::middleware::timeout::TimeoutMiddleware;
use crate::controller::middleware::tracing::TracingMiddleware;
use crate::controller::middleware::Middleware;

pub fn default_middleware() -> Vec<Box<dyn Middleware>> {
    vec![
        Box::new(SensitiveRequestHeadersMiddleware),
        Box::new(SensitiveResponseHeadersMiddleware),
        Box::new(SetRequestIdMiddleware),
        Box::new(PropagateRequestIdMiddleware),
        Box::new(TracingMiddleware),
        Box::new(CatchPanicMiddleware),
        Box::new(RequestDecompressionMiddleware),
        Box::new(TimeoutMiddleware),
        Box::new(RequestBodyLimitMiddleware),
    ]
}
