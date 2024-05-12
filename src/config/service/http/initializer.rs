use crate::app_context::AppContext;
use crate::config::app_config::CustomConfig;
use crate::service::http::initializer::normalize_path::NormalizePathConfig;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Initializer {
    pub default_enable: bool,
    pub normalize_path: InitializerConfig<NormalizePathConfig>,
    /// Allows providing configs for custom initializers. Any configs that aren't pre-defined above
    /// will be collected here.
    ///
    /// # Examples
    ///
    /// ```toml
    /// [initializer.foo]
    /// enable = true
    /// priority = 10
    /// x = "y"
    /// ```
    ///
    /// This will be parsed as:
    /// ```raw
    /// Initializer#custom: {
    ///     "foo": {
    ///         InitializerConfig#common: {
    ///             enable: true,
    ///             priority: 10
    ///         },
    ///         InitializerConfig<CustomConfig>#custom: {
    ///             config: {
    ///                 "x": "y"
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(flatten)]
    pub custom: BTreeMap<String, InitializerConfig<CustomConfig>>,
}

impl Default for Initializer {
    fn default() -> Self {
        let normalize_path: InitializerConfig<NormalizePathConfig> = Default::default();
        let normalize_path = normalize_path.set_priority(PRIORITY_LAST);

        Self {
            default_enable: true,
            normalize_path,
            custom: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the initializer, vs the initializer being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Initializer#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    pub priority: i32,
}

impl CommonConfig {
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn enabled<S>(&self, context: &AppContext<S>) -> bool {
        self.enable.unwrap_or(
            context
                .config()
                .service
                .http
                .custom
                .initializer
                .default_enable,
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct InitializerConfig<T: Default> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

impl<T: Default> InitializerConfig<T> {
    pub fn set_priority(mut self, priority: i32) -> Self {
        self.common = self.common.set_priority(priority);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn custom_config() {
        // Note: since we're parsing into a Initializer config struct directly, we don't
        // need to prefix `foo` with `initializer`. If we want to actually provide custom initializer
        // configs, the table key will need to be `[initializer.foo]`.
        let config = r#"
        [foo]
        enable = true
        priority = 10
        x = "y"
        "#;
        let config: Initializer = toml::from_str(config).unwrap();

        assert!(config.custom.contains_key("foo"));

        let config = config.custom.get("foo").unwrap();
        assert_eq!(config.common.enable, Some(true));
        assert_eq!(config.common.priority, 10);

        assert!(config.custom.config.contains_key("x"));
        let x = config.custom.config.get("x").unwrap();
        assert_eq!(x, &Value::String("y".to_string()));
    }
}
