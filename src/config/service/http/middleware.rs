use crate::app_context::AppContext;
use crate::config::app_config::CustomConfig;
use crate::service::http::middleware::catch_panic::CatchPanicConfig;
use crate::service::http::middleware::compression::{
    RequestDecompressionConfig, ResponseCompressionConfig,
};
use crate::service::http::middleware::request_id::{PropagateRequestIdConfig, SetRequestIdConfig};
use crate::service::http::middleware::sensitive_headers::{
    SensitiveRequestHeadersConfig, SensitiveResponseHeadersConfig,
};
use crate::service::http::middleware::size_limit::SizeLimitConfig;
use crate::service::http::middleware::timeout::TimeoutConfig;
use crate::service::http::middleware::tracing::TracingConfig;
use crate::util::serde_util;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Middleware {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[serde(
        deserialize_with = "deserialize_sensitive_request_headers",
        default = "default_sensitive_request_headers"
    )]
    pub sensitive_request_headers: MiddlewareConfig<SensitiveRequestHeadersConfig>,

    #[serde(
        deserialize_with = "deserialize_sensitive_response_headers",
        default = "default_sensitive_response_headers"
    )]
    pub sensitive_response_headers: MiddlewareConfig<SensitiveResponseHeadersConfig>,

    #[serde(
        deserialize_with = "deserialize_set_request_id",
        default = "default_set_request_id"
    )]
    pub set_request_id: MiddlewareConfig<SetRequestIdConfig>,

    #[serde(
        deserialize_with = "deserialize_propagate_request_id",
        default = "default_propagate_request_id"
    )]
    pub propagate_request_id: MiddlewareConfig<PropagateRequestIdConfig>,

    #[serde(deserialize_with = "deserialize_tracing", default = "default_tracing")]
    pub tracing: MiddlewareConfig<TracingConfig>,

    #[serde(
        deserialize_with = "deserialize_catch_panic",
        default = "default_catch_panic"
    )]
    pub catch_panic: MiddlewareConfig<CatchPanicConfig>,

    #[serde(
        deserialize_with = "deserialize_response_compression",
        default = "default_response_compression"
    )]
    pub response_compression: MiddlewareConfig<ResponseCompressionConfig>,

    #[serde(
        deserialize_with = "deserialize_request_decompression",
        default = "default_request_decompression"
    )]
    pub request_decompression: MiddlewareConfig<RequestDecompressionConfig>,

    #[serde(deserialize_with = "deserialize_timeout", default = "default_timeout")]
    pub timeout: MiddlewareConfig<TimeoutConfig>,

    #[serde(
        deserialize_with = "deserialize_size_limit",
        default = "default_size_limit"
    )]
    pub size_limit: MiddlewareConfig<SizeLimitConfig>,
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
    ///             config: {
    ///                 "x": "y"
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(flatten)]
    pub custom: BTreeMap<String, MiddlewareConfig<CustomConfig>>,
}

impl Default for Middleware {
    fn default() -> Self {
        Self {
            default_enable: default_true(),
            sensitive_request_headers: default_sensitive_request_headers(),
            sensitive_response_headers: default_sensitive_response_headers(),
            set_request_id: default_set_request_id(),
            propagate_request_id: default_propagate_request_id(),
            tracing: default_tracing(),
            catch_panic: default_catch_panic(),
            response_compression: default_response_compression(),
            request_decompression: default_request_decompression(),
            timeout: default_timeout(),
            size_limit: default_size_limit(),
            custom: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the middleware, vs the middleware being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Middleware#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub enable: Option<bool>,
    pub priority: i32,
}

impl CommonConfig {
    pub fn enabled<S>(&self, context: &AppContext<S>) -> bool {
        self.enable.unwrap_or(
            context
                .config()
                .service
                .http
                .custom
                .middleware
                .default_enable,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MiddlewareConfig<T> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

// This fun boilerplate allows the user to
// 1. Partially override a config without needing to provide all of the required values for the config
// 2. Prevent a type's `Default` implementation from being used and overriding the default we
//    actually want. For example, we provide a default for the `priority` fields, and we want that
//    value to be used if the user doesn't provide one, not the type's default (`0` in this case).
//
// See: https://users.rust-lang.org/t/serde-default-value-for-struct-field-depending-on-parent/73452/2
//
// This is mainly needed because all of the middleware share a struct for their common configs,
// so we can't simply set a default on the field directly with a serde annotation.
// An alternative implementation could be to have different structs for each middleware's common
// config instead of sharing a struct type. However, that would still require a lot of boilerplate.

struct Priorities {
    sensitive_request_headers: i32,
    sensitive_response_headers: i32,
    set_request_id: i32,
    propagate_request_id: i32,
    tracing: i32,
    catch_panic: i32,
    response_compression: i32,
    request_decompression: i32,
    timeout: i32,
    size_limit: i32,
}

impl Default for Priorities {
    fn default() -> Self {
        let mut priority = PRIORITY_FIRST;
        let sensitive_request_headers = priority;

        priority += 10;
        let set_request_id = priority;

        priority += 10;
        let tracing = priority;

        priority += 10;
        let size_limit = priority;

        priority += 10;
        let request_decompression = priority;

        // Somewhere in the middle, order doesn't particularly matter
        let catch_panic = 0;
        let response_compression = 0;
        let timeout = 0;

        // Before response middlewares
        let mut priority = PRIORITY_LAST;
        let sensitive_response_headers = priority;

        priority -= 10;
        let propagate_request_id = priority;

        Self {
            sensitive_request_headers,
            set_request_id,
            tracing,
            size_limit,
            request_decompression,
            catch_panic,
            response_compression,
            timeout,
            sensitive_response_headers,
            propagate_request_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct PartialCommonConfig {
    pub enable: Option<bool>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IncompleteMiddlewareConfig<T> {
    #[serde(flatten)]
    pub common: PartialCommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

fn deserialize_sensitive_request_headers<'de, D, T>(
    deserializer: D,
) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config(
        Priorities::default().sensitive_request_headers,
    ))
}

fn default_sensitive_request_headers() -> MiddlewareConfig<SensitiveRequestHeadersConfig> {
    deserialize_sensitive_request_headers(serde_util::empty_json_object()).unwrap()
}

fn deserialize_sensitive_response_headers<'de, D, T>(
    deserializer: D,
) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config(
        Priorities::default().sensitive_response_headers,
    ))
}

fn default_sensitive_response_headers() -> MiddlewareConfig<SensitiveResponseHeadersConfig> {
    deserialize_sensitive_response_headers(serde_util::empty_json_object()).unwrap()
}

fn deserialize_set_request_id<'de, D, T>(deserializer: D) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().set_request_id))
}

fn default_set_request_id() -> MiddlewareConfig<SetRequestIdConfig> {
    deserialize_set_request_id(serde_util::empty_json_object()).unwrap()
}

fn deserialize_propagate_request_id<'de, D, T>(
    deserializer: D,
) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().propagate_request_id))
}

fn default_propagate_request_id() -> MiddlewareConfig<PropagateRequestIdConfig> {
    deserialize_propagate_request_id(serde_util::empty_json_object()).unwrap()
}

fn deserialize_tracing<'de, D, T>(deserializer: D) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().tracing))
}

fn default_tracing() -> MiddlewareConfig<TracingConfig> {
    deserialize_tracing(serde_util::empty_json_object()).unwrap()
}

fn deserialize_catch_panic<'de, D, T>(deserializer: D) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().catch_panic))
}

fn default_catch_panic() -> MiddlewareConfig<CatchPanicConfig> {
    deserialize_catch_panic(serde_util::empty_json_object()).unwrap()
}

fn deserialize_response_compression<'de, D, T>(
    deserializer: D,
) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().response_compression))
}

fn default_response_compression() -> MiddlewareConfig<ResponseCompressionConfig> {
    deserialize_response_compression(serde_util::empty_json_object()).unwrap()
}

fn deserialize_request_decompression<'de, D, T>(
    deserializer: D,
) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config(
        Priorities::default().request_decompression,
    ))
}

fn default_request_decompression() -> MiddlewareConfig<RequestDecompressionConfig> {
    deserialize_request_decompression(serde_util::empty_json_object()).unwrap()
}

fn deserialize_timeout<'de, D, T>(deserializer: D) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().timeout))
}

fn default_timeout() -> MiddlewareConfig<TimeoutConfig> {
    deserialize_timeout(serde_util::empty_json_object()).unwrap()
}

fn deserialize_size_limit<'de, D, T>(deserializer: D) -> Result<MiddlewareConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().size_limit))
}

fn default_size_limit() -> MiddlewareConfig<SizeLimitConfig> {
    deserialize_size_limit(serde_util::empty_json_object()).unwrap()
}

fn map_empty_config<T>(
    default_priority: i32,
) -> impl FnOnce(IncompleteMiddlewareConfig<T>) -> MiddlewareConfig<T> {
    move |IncompleteMiddlewareConfig { common, custom }| MiddlewareConfig {
        common: CommonConfig {
            enable: common.enable,
            priority: common.priority.unwrap_or(default_priority),
        },
        custom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_config::AppConfig;
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

        let context = AppContext::<()>::test(Some(config), None).unwrap();

        let common_config = CommonConfig {
            enable,
            priority: 0,
        };

        // Act/Assert
        assert_eq!(common_config.enabled(&context), expected_enabled);
    }
}

#[cfg(test)]
mod deserialize_tests {
    use super::*;
    use crate::util::test_util::TestCase;
    use insta::assert_toml_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case("")]
    #[case(
        r#"
        [sensitive-request-headers]
        enable = false
        [sensitive-response-headers]
        enable = false
        [set-request-id]
        enable = false
        [propagate-request-id]
        enable = false
        [tracing]
        enable = false
        [catch-panic]
        enable = false
        [response-compression]
        enable = false
        [request-decompression]
        enable = false
        [timeout]
        enable = false
        [size-limit]
        enable = false
        "#
    )]
    #[case(
        r#"
        default-enable = false
        [sensitive-request-headers]
        priority = -1
        [sensitive-response-headers]
        priority = 0
        [set-request-id]
        priority = 1
        [propagate-request-id]
        priority = 2
        [tracing]
        priority = 3
        [catch-panic]
        priority = 4
        [response-compression]
        priority = 5
        [request-decompression]
        priority = 6
        [timeout]
        priority = 7
        [size-limit]
        priority = 8
        "#
    )]
    #[case(
        r#"
        [foo]
        enable = true
        priority = 10
        x = "y"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn middleware(_case: TestCase, #[case] config: &str) {
        let middleware: Middleware = toml::from_str(config).unwrap();

        assert_toml_snapshot!(middleware);
    }
}
