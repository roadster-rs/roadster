use crate::app::context::AppContext;
use crate::config::CustomConfig;
use crate::service::http::initializer::normalize_path::NormalizePathConfig;
use crate::util::serde::default_true;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Initializer {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[validate(nested)]
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
    #[validate(nested)]
    pub custom: BTreeMap<String, InitializerConfig<CustomConfig>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the initializer, vs the initializer being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Initializer#default_enable`.
    #[serde(default)]
    pub enable: Option<bool>,
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
                .initializer
                .default_enable,
        )
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct InitializerConfig<T: Validate> {
    #[serde(flatten)]
    #[validate(nested)]
    pub common: CommonConfig,
    #[serde(flatten)]
    #[validate(nested)]
    pub custom: T,
}
