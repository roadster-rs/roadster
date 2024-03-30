use crate::app_context::AppContext;
use crate::controller::middleware::Middleware;
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
impl Middleware for ResponseCompressionMiddleware {
    fn name(&self) -> String {
        "response-compression".to_string()
    }

    fn enabled(&self, context: &AppContext) -> bool {
        context
            .config
            .middleware
            .response_compression
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext) -> i32 {
        context
            .config
            .middleware
            .response_compression
            .common
            .priority
    }

    fn install(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        let router = router.layer(CompressionLayer::new());

        Ok(router)
    }
}

pub struct RequestDecompressionMiddleware;
impl Middleware for RequestDecompressionMiddleware {
    fn name(&self) -> String {
        "request-decompression".to_string()
    }

    fn enabled(&self, context: &AppContext) -> bool {
        context
            .config
            .middleware
            .request_decompression
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext) -> i32 {
        context
            .config
            .middleware
            .request_decompression
            .common
            .priority
    }

    fn install(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        let router = router.layer(RequestDecompressionLayer::new());

        Ok(router)
    }
}
