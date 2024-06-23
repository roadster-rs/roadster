use crate::app::context::AppContext;
use crate::config::app_config::CustomConfig;
use crate::service::http::initializer::normalize_path::NormalizePathConfig;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Initializer {
    #[serde(default = "default_true")]
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
    ///             "x": "y"
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(flatten)]
    pub custom: BTreeMap<String, InitializerConfig<CustomConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the initializer, vs the initializer being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Initializer#default_enable`.
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
                .initializer
                .default_enable,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct InitializerConfig<T> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}
