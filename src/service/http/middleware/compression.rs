use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::Router;
use serde_derive::{Deserialize, Serialize};

use tower_http::compression::CompressionLayer;
use tower_http::decompression::RequestDecompressionLayer;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct ResponseCompressionConfig {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct RequestDecompressionConfig {}

pub struct ResponseCompressionMiddleware;
impl<S> Middleware<S> for ResponseCompressionMiddleware {
    fn name(&self) -> String {
        "response-compression".to_string()
    }

    fn enabled(&self, context: &AppContext, _state: &S) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .response_compression
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext, _state: &S) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .response_compression
            .common
            .priority
    }

    fn install(&self, router: Router, _context: &AppContext, _state: &S) -> anyhow::Result<Router> {
        let router = router.layer(CompressionLayer::new());

        Ok(router)
    }
}

pub struct RequestDecompressionMiddleware;
impl<S> Middleware<S> for RequestDecompressionMiddleware {
    fn name(&self) -> String {
        "request-decompression".to_string()
    }

    fn enabled(&self, context: &AppContext, _state: &S) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext, _state: &S) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .common
            .priority
    }

    fn install(&self, router: Router, _context: &AppContext, _state: &S) -> anyhow::Result<Router> {
        let router = router.layer(RequestDecompressionLayer::new());

        Ok(router)
    }
}
