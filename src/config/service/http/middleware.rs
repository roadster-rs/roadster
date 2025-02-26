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
use crate::service::http::middleware::tracing::TracingConfig;
use crate::service::http::middleware::tracing::req_res_logging::RequestResponseLoggingConfig;
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

    #[validate(nested)]
    pub sensitive_request_headers: MiddlewareConfig<SensitiveRequestHeadersConfig>,

    #[validate(nested)]
    pub sensitive_response_headers: MiddlewareConfig<SensitiveResponseHeadersConfig>,

    #[validate(nested)]
    pub set_request_id: MiddlewareConfig<SetRequestIdConfig>,

    #[validate(nested)]
    pub propagate_request_id: MiddlewareConfig<PropagateRequestIdConfig>,

    #[validate(nested)]
    pub tracing: MiddlewareConfig<TracingConfig>,

    #[validate(nested)]
    pub catch_panic: MiddlewareConfig<CatchPanicConfig>,

    #[validate(nested)]
    pub response_compression: MiddlewareConfig<ResponseCompressionConfig>,

    #[validate(nested)]
    pub request_decompression: MiddlewareConfig<RequestDecompressionConfig>,

    #[validate(nested)]
    pub timeout: MiddlewareConfig<TimeoutConfig>,

    #[validate(nested)]
    pub size_limit: MiddlewareConfig<SizeLimitConfig>,

    #[validate(nested)]
    pub cors: MiddlewareConfig<CorsConfig>,

    #[validate(nested)]
    pub request_response_logging: MiddlewareConfig<RequestResponseLoggingConfig>,

    #[validate(nested)]
    pub cache_control: MiddlewareConfig<CacheControlConfig>,

    #[validate(nested)]
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
    #[validate(nested)]
    pub custom: BTreeMap<String, MiddlewareConfig<CustomConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
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
pub struct MiddlewareConfig<T: Validate> {
    #[serde(flatten)]
    #[validate(nested)]
    pub common: CommonConfig,
    #[serde(flatten)]
    #[validate(nested)]
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
