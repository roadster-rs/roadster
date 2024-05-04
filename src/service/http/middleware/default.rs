use crate::service::http::middleware::catch_panic::CatchPanicMiddleware;
use crate::service::http::middleware::compression::RequestDecompressionMiddleware;
use crate::service::http::middleware::request_id::{
    PropagateRequestIdMiddleware, SetRequestIdMiddleware,
};
use crate::service::http::middleware::sensitive_headers::{
    SensitiveRequestHeadersMiddleware, SensitiveResponseHeadersMiddleware,
};
use crate::service::http::middleware::size_limit::RequestBodyLimitMiddleware;
use crate::service::http::middleware::timeout::TimeoutMiddleware;
use crate::service::http::middleware::tracing::TracingMiddleware;
use crate::service::http::middleware::Middleware;

pub fn default_middleware<S>() -> Vec<Box<dyn Middleware<S>>> {
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
