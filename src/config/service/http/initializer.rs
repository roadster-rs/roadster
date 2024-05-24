use crate::app_context::AppContext;
use crate::config::app_config::CustomConfig;
use crate::service::http::initializer::normalize_path::NormalizePathConfig;
use crate::util::serde_util;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub const PRIORITY_FIRST: i32 = -10_000;
pub const PRIORITY_LAST: i32 = 10_000;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct Initializer {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[serde(
        deserialize_with = "deserialize_normalize_path",
        default = "default_normalize_path"
    )]
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
        Self {
            default_enable: default_true(),
            normalize_path: default_normalize_path(),
            custom: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
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
pub struct InitializerConfig<T> {
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
// This is mainly needed because all of the initializers share a struct for their common configs,
// so we can't simply set a default on the field directly with a serde annotation.
// An alternative implementation could be to have different structs for each initializer's common
// config instead of sharing a struct type. However, that would still require a lot of boilerplate.

struct Priorities {
    normalize_path: i32,
}

impl Default for Priorities {
    fn default() -> Self {
        let normalize_path = PRIORITY_LAST;

        Self { normalize_path }
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
pub struct IncompleteInitializerConfig<T> {
    #[serde(flatten)]
    pub common: PartialCommonConfig,
    #[serde(flatten)]
    pub custom: T,
}

fn deserialize_normalize_path<'de, D, T>(deserializer: D) -> Result<InitializerConfig<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config(Priorities::default().normalize_path))
}

fn default_normalize_path() -> InitializerConfig<NormalizePathConfig> {
    deserialize_normalize_path(serde_util::empty_json_object()).unwrap()
}

fn map_empty_config<T>(
    default_priority: i32,
) -> impl FnOnce(IncompleteInitializerConfig<T>) -> InitializerConfig<T> {
    move |IncompleteInitializerConfig { common, custom }| InitializerConfig {
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
        [normalize-path]
        enable = false
        "#
    )]
    #[case(
        r#"
        [normalize-path]
        priority = 1234
        "#
    )]
    #[case(
        r#"
        default-enable = false
        "#
    )]
    #[case(
        r#"
        default-enable = false
        [normalize-path]
        enable = false
        priority = 1234
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
    fn initializer(_case: TestCase, #[case] config: &str) {
        let initializer: Initializer = toml::from_str(config).unwrap();

        assert_toml_snapshot!(initializer);
    }
}
