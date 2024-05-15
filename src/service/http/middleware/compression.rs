#[mockall_double::double]
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
impl<S: Send + Sync + 'static> Middleware<S> for ResponseCompressionMiddleware {
    fn name(&self) -> String {
        "response-compression".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
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

    fn priority(&self, context: &AppContext<S>) -> i32 {
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

    fn install(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        let router = router.layer(CompressionLayer::new());

        Ok(router)
    }
}

pub struct RequestDecompressionMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for RequestDecompressionMiddleware {
    fn name(&self) -> String {
        "request-decompression".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
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

    fn priority(&self, context: &AppContext<S>) -> i32 {
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

    fn install(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        let router = router.layer(RequestDecompressionLayer::new());

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    fn response_compression_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .middleware
            .response_compression
            .common
            .enable = enable;

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = ResponseCompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 0)]
    #[case(Some(1234), 1234)]
    fn response_compression_priority(
        #[case] override_priority: Option<i32>,
        #[case] expected_priority: i32,
    ) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .response_compression
                .common
                .priority = priority;
        }

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = ResponseCompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    fn request_decompression_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .common
            .enable = enable;

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = RequestDecompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, -9960)]
    #[case(Some(1234), 1234)]
    fn request_decompression_priority(
        #[case] override_priority: Option<i32>,
        #[case] expected_priority: i32,
    ) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .request_decompression
                .common
                .priority = priority;
        }

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = RequestDecompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
