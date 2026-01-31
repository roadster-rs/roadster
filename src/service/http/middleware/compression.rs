use crate::app::context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::Router;
use axum::http::Response;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::compression::predicate::{NotForContentType, SizeAbove};
use tower_http::compression::{CompressionLayer, Predicate};
use tower_http::decompression::RequestDecompressionLayer;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ResponseCompressionConfig {
    pub min_size_bytes: u16,
    pub level: CompressionLevel,
    /// Additional content types that should not be compressed in addition to the ones
    /// specified by [`tower_http::compression::DefaultPredicate`].
    pub not_for_content_types: Vec<String>,
    #[serde(flatten)]
    #[validate(nested)]
    pub accept: AcceptEncoding,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct RequestDecompressionConfig {
    /// Whether to pass through the request even when the encoding is not supported.
    pub pass_through_unaccepted: bool,
    #[serde(flatten)]
    #[validate(nested)]
    pub accept: AcceptEncoding,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct AcceptEncoding {
    pub br: bool,
    pub deflate: bool,
    pub gzip: bool,
    pub zstd: bool,
}

/// Serializable version of [`tower_http::CompressionLevel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CompressionLevel {
    Fastest,
    Best,
    Default,
    #[serde(untagged)]
    Precise(i32),
}

impl From<CompressionLevel> for tower_http::CompressionLevel {
    fn from(value: CompressionLevel) -> Self {
        match value {
            CompressionLevel::Fastest => tower_http::CompressionLevel::Fastest,
            CompressionLevel::Best => tower_http::CompressionLevel::Best,
            CompressionLevel::Default => tower_http::CompressionLevel::Default,
            CompressionLevel::Precise(value) => tower_http::CompressionLevel::Precise(value),
        }
    }
}

pub struct ResponseCompressionMiddleware;
impl<S> Middleware<S> for ResponseCompressionMiddleware
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        "response-compression".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .response_compression
            .common
            .enabled(state)
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .response_compression
            .common
            .priority
    }

    fn install(&self, state: &S, router: Router) -> Result<Router, Self::Error> {
        let context = AppContext::from_ref(state);
        let config = context.config();
        let middleware_config = &config
            .service
            .http
            .custom
            .middleware
            .response_compression
            .custom;

        let layer = CompressionLayer::new()
            .quality(middleware_config.level.clone().into())
            .br(middleware_config.accept.br)
            .deflate(middleware_config.accept.deflate)
            .gzip(middleware_config.accept.gzip)
            .zstd(middleware_config.accept.zstd)
            .compress_when(
                SizeAbove::new(middleware_config.min_size_bytes)
                    .and(NotForContentType::GRPC)
                    .and(NotForContentType::IMAGES)
                    .and(NotForContentType::SSE)
                    .and(MultiContentTypePredicate::new(
                        middleware_config.not_for_content_types.clone(),
                    )),
            );

        let router = router.layer(layer);

        Ok(router)
    }
}

#[derive(Clone)]
#[non_exhaustive]
struct MultiContentTypePredicate {
    not_for_content_types: Arc<Vec<String>>,
}

impl MultiContentTypePredicate {
    fn new(not_for_content_types: Vec<String>) -> Self {
        Self {
            not_for_content_types: Arc::new(not_for_content_types),
        }
    }
}

impl Predicate for MultiContentTypePredicate {
    fn should_compress<B>(&self, response: &Response<B>) -> bool
    where
        B: http_body::Body,
    {
        if self.not_for_content_types.is_empty() {
            return true;
        }

        let content_type = response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();

        !self
            .not_for_content_types
            .iter()
            .any(|not_for_content_type| content_type.starts_with(not_for_content_type))
    }
}

pub struct RequestDecompressionMiddleware;
impl<S> Middleware<S> for RequestDecompressionMiddleware
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        "request-decompression".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .common
            .enabled(state)
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .common
            .priority
    }

    fn install(&self, state: &S, router: Router) -> Result<Router, Self::Error> {
        let context = AppContext::from_ref(state);
        let config = context.config();
        let middleware_config = &config
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .custom;

        let layer = RequestDecompressionLayer::new()
            .pass_through_unaccepted(middleware_config.pass_through_unaccepted)
            .br(middleware_config.accept.br)
            .deflate(middleware_config.accept.deflate)
            .gzip(middleware_config.accept.gzip)
            .zstd(middleware_config.accept.zstd);

        let router = router.layer(layer);

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn response_compression_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .middleware
            .response_compression
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = ResponseCompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 0)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn response_compression_priority(
        #[case] override_priority: Option<i32>,
        #[case] expected_priority: i32,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
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

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = ResponseCompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn request_decompression_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .middleware
            .request_decompression
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = RequestDecompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, -9960)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn request_decompression_priority(
        #[case] override_priority: Option<i32>,
        #[case] expected_priority: i32,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
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

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = RequestDecompressionMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
