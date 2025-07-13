use crate::app::context::AppContext;
use crate::config::CustomConfig;
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub(crate) fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct LifecycleHandler {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[cfg(feature = "db-sql")]
    #[validate(nested)]
    pub db_migration: LifecycleHandlerConfig<crate::config::EmptyConfig>,

    #[cfg(feature = "db-sql")]
    #[validate(nested)]
    pub db_graceful_shutdown: LifecycleHandlerConfig<crate::config::EmptyConfig>,

    /// Allows providing configs for custom lifecycle handlers. Any configs that aren't pre-defined
    /// above will be collected here.
    ///
    /// # Examples
    ///
    /// ```toml
    /// [lifecycle-handler.foo]
    /// enable = true
    /// x = "y"
    /// ```
    ///
    /// This will be parsed as:
    /// ```raw
    /// LifecycleHandler#custom: {
    ///     "foo": {
    ///         LifecycleHandlerConfig#common: {
    ///             enable: true,
    ///             priority: 10
    ///         },
    ///         LifecycleHandlerConfig<CustomConfig>#custom: {
    ///             "x": "y"
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(flatten)]
    #[validate(nested)]
    pub custom: BTreeMap<String, LifecycleHandlerConfig<CustomConfig>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the lifecycle handler, vs the lifecycle handler being enabled/disabled by default.
    // If this is `None`, the value will match the value of `LifecycleHandler#default_enable`.
    pub enable: Option<bool>,
    pub priority: i32,
}

impl CommonConfig {
    pub fn enabled(&self, context: &AppContext) -> bool {
        self.enable
            .unwrap_or(context.config().lifecycle_handler.default_enable)
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct LifecycleHandlerConfig<T: Validate> {
    #[serde(flatten, default)]
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
        config.lifecycle_handler.default_enable = default_enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let common_config = CommonConfig {
            enable,
            priority: 0,
        };

        // Act/Assert
        assert_eq!(common_config.enabled(&context), expected_enabled);
    }
}
