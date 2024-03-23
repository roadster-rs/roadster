use crate::app_context::AppContext;
use crate::controller::middleware::catch_panic::CatchPanicConfig;
use crate::controller::middleware::request_id::{PropagateRequestIdConfig, SetRequestIdConfig};
use crate::controller::middleware::sensitive_headers::{
    SensitiveRequestHeadersConfig, SensitiveResponseHeadersConfig,
};
use crate::controller::middleware::tracing::TracingConfig;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use toml::Value;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Middleware {
    pub default_enable: bool,
    pub sensitive_request_headers: MiddlewareConfig<SensitiveRequestHeadersConfig>,
    pub sensitive_response_headers: MiddlewareConfig<SensitiveResponseHeadersConfig>,
    pub set_request_id: MiddlewareConfig<SetRequestIdConfig>,
    pub propagate_request_id: MiddlewareConfig<PropagateRequestIdConfig>,
    pub tracing: MiddlewareConfig<TracingConfig>,
    pub catch_panic: MiddlewareConfig<CatchPanicConfig>,
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
    ///         MiddlewareConfig<CustomMiddlewareConfig>#custom: {
    ///             config: {
    ///                 "x": "y"
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    // Todo: consolidate custom settings for both middleware an initializers?
    #[serde(flatten)]
    pub custom: BTreeMap<String, MiddlewareConfig<CustomMiddlewareConfig>>,
}

impl Default for Middleware {
    fn default() -> Self {
        // Before request middlewares
        let mut priority = PRIORITY_FIRST;
        let sensitive_request_headers: MiddlewareConfig<SensitiveRequestHeadersConfig> =
            Default::default();
        let sensitive_request_headers = sensitive_request_headers.set_priority(priority);

        priority += 10;
        let set_request_id: MiddlewareConfig<SetRequestIdConfig> = Default::default();
        let set_request_id = set_request_id.set_priority(priority);

        // Somewhere in the middle, order doesn't particularly matter
        let tracing: MiddlewareConfig<TracingConfig> = Default::default();
        let catch_panic: MiddlewareConfig<CatchPanicConfig> = Default::default();

        // Before response middlewares
        let mut priority = PRIORITY_LAST;
        let sensitive_response_headers: MiddlewareConfig<SensitiveResponseHeadersConfig> =
            Default::default();
        let sensitive_response_headers = sensitive_response_headers.set_priority(priority);

        priority -= 10;
        let propagate_request_id: MiddlewareConfig<PropagateRequestIdConfig> = Default::default();
        let propagate_request_id = propagate_request_id.set_priority(priority);

        Self {
            default_enable: true,
            sensitive_request_headers,
            sensitive_response_headers,
            set_request_id,
            propagate_request_id,
            tracing,
            catch_panic,
            custom: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the middleware, vs the middleware being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Middleware#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    pub priority: i32,
}

impl CommonConfig {
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn enabled(&self, context: &AppContext) -> bool {
        self.enable
            .unwrap_or(context.config.middleware.default_enable)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct MiddlewareConfig<T: Default> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

impl<T: Default> MiddlewareConfig<T> {
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.common = self.common.set_priority(priority);
        self
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CustomMiddlewareConfig {
    #[serde(flatten)]
    pub config: BTreeMap<String, Value>,
}

#[cfg(test)]
mod test {
    use toml::Value;

    #[test]
    fn test_custom_config() {
        // Note: since we're parsing into a Middleware config struct directly, we don't
        // need to prefix `foo` with `middleware`. If we want to actually provide custom middleware
        // configs, the table key will need to be `[middleware.foo]`.
        let config = r#"
        [foo]
        enable = true
        priority = 10
        x = "y"
        "#;
        let config: crate::config::middleware::Middleware = toml::from_str(config).unwrap();

        assert!(config.custom.contains_key("foo"));

        let config = config.custom.get("foo").unwrap();
        assert_eq!(config.common.enable, Some(true));
        assert_eq!(config.common.priority, 10);

        assert!(config.custom.config.contains_key("x"));
        let x = config.custom.config.get("x").unwrap();
        assert_eq!(x, &Value::String("y".to_string()));
    }
}
