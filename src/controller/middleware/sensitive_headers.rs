use crate::app_context::AppContext;
use crate::controller::middleware::Middleware;
use axum::http::{header, HeaderName};
use axum::Router;
use lazy_static::lazy_static;
use std::sync::Arc;
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};

lazy_static! {
    pub static ref SENSITIVE_HEADERS: Arc<[HeaderName]> = Arc::new([
        header::AUTHORIZATION,
        header::PROXY_AUTHORIZATION,
        header::COOKIE,
        header::SET_COOKIE,
    ]);
}

pub struct SensitiveRequestHeadersMiddleware;

impl Middleware for SensitiveRequestHeadersMiddleware {
    fn name(&self) -> String {
        "sensitive-request-headers".to_string()
    }

    fn install(&self, router: Router, _context: &AppContext) -> Router {
        router.layer(SetSensitiveRequestHeadersLayer::from_shared(
            SENSITIVE_HEADERS.clone(),
        ))
    }
}

pub struct SensitiveResponseHeadersMiddleware;

impl Middleware for SensitiveResponseHeadersMiddleware {
    fn name(&self) -> String {
        "sensitive-response-headers".to_string()
    }

    fn install(&self, router: Router, _context: &AppContext) -> Router {
        router.layer(SetSensitiveResponseHeadersLayer::from_shared(
            SENSITIVE_HEADERS.clone(),
        ))
    }
}
