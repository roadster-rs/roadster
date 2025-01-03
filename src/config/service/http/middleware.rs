use crate::app::context::AppContext;
use crate::config::CustomConfig;
use crate::service::http::middleware::cache_control::CacheControlConfig;
use crate::service::http::middleware::catch_panic::CatchPanicConfig;
use crate::service::http::middleware::compression::{
    RequestDecompressionConfig, ResponseCompressionConfig,
};
use crate::service::http::middleware::cors::CorsConfig;
use crate::service::http::middleware::etag::EtagConfig;
use crate::service::http::middleware::request_id::{PropagateRequestIdConfig, SetRequestIdConfig};
use crate::service::http::middleware::sensitive_headers::{
    SensitiveRequestHeadersConfig, SensitiveResponseHeadersConfig,
};
use crate::service::http::middleware::size_limit::SizeLimitConfig;
use crate::service::http::middleware::timeout::TimeoutConfig;
use crate::service::http::middleware::tracing::req_res_logging::RequestResponseLoggingConfig;
use crate::service::http::middleware::tracing::TracingConfig;
use crate::util::serde::default_true;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Middleware {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    pub sensitive_request_headers: MiddlewareConfig<SensitiveRequestHeadersConfig>,

    pub sensitive_response_headers: MiddlewareConfig<SensitiveResponseHeadersConfig>,

    pub set_request_id: MiddlewareConfig<SetRequestIdConfig>,

    pub propagate_request_id: MiddlewareConfig<PropagateRequestIdConfig>,

    pub tracing: MiddlewareConfig<TracingConfig>,

    pub catch_panic: MiddlewareConfig<CatchPanicConfig>,

    pub response_compression: MiddlewareConfig<ResponseCompressionConfig>,

    pub request_decompression: MiddlewareConfig<RequestDecompressionConfig>,

    pub timeout: MiddlewareConfig<TimeoutConfig>,

    pub size_limit: MiddlewareConfig<SizeLimitConfig>,

    pub cors: MiddlewareConfig<CorsConfig>,

    pub request_response_logging: MiddlewareConfig<RequestResponseLoggingConfig>,

    pub cache_control: MiddlewareConfig<CacheControlConfig>,

    pub etag: MiddlewareConfig<EtagConfig>,

    /// Allows providing configs for custom middleware. Any configs that aren't pre-defined above
    /// will be collected here.
    ///
    /// # Examples
    ///
    /// ```toml
    /// [middleware.foo]
    /// enable = true
    /// priority = 10
    /// x = "y"
    /// ```
    ///
    /// This will be parsed as:
    /// ```raw
    /// Middleware#custom: {
    ///     "foo": {
    ///         MiddlewareConfig#common: {
    ///             enable: true,
    ///             priority: 10
    ///         },
    ///         MiddlewareConfig<CustomConfig>#custom: {
    ///             "x": "y"
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(flatten)]
    pub custom: BTreeMap<String, MiddlewareConfig<CustomConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the middleware, vs the middleware being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Middleware#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub enable: Option<bool>,
    #[serde(default)]
    pub priority: i32,
}

impl CommonConfig {
    pub fn enabled<S>(&self, state: &S) -> bool
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        self.enable.unwrap_or(
            AppContext::from_ref(state)
                .config()
                .service
                .http
                .custom
                .middleware
                .default_enable,
        )
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct MiddlewareConfig<T> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(true, None, true)]
    #[case(true, Some(true), true)]
    #[case(true, Some(false), false)]
    #[case(false, None, false)]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn common_config_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let common_config = CommonConfig {
            enable,
            priority: 0,
        };

        // Act/Assert
        assert_eq!(common_config.enabled(&context), expected_enabled);
    }
}
