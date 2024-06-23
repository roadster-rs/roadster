use crate::app::context::AppContext;
use crate::config::app_config::CustomConfig;
use crate::util::serde_util::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

pub fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct HealthCheck {
    #[serde(default = "default_true")]
    pub default_enable: bool,
    #[cfg(feature = "db-sql")]
    pub database: HealthCheckConfig<()>,
    #[cfg(feature = "sidekiq")]
    pub sidekiq: HealthCheckConfig<()>,
    /// Allows providing configs for custom health checks. Any configs that aren't pre-defined above
    /// will be collected here.
    ///
    /// # Examples
    ///
    /// ```toml
    /// [health-check.foo]
    /// enable = true
    /// x = "y"
    /// ```
    ///
    /// This will be parsed as:
    /// ```raw
    /// HealthCheck#custom: {
    ///     "foo": {
    ///         HealthCheckConfig#common: {
    ///             enable: true,
    ///             priority: 10
    ///         },
    ///         HealthCheckConfig<CustomConfig>#custom: {
    ///             "x": "y"
    ///         }
    ///     }
    /// }
    /// ```
    #[serde(flatten)]
    pub custom: BTreeMap<String, HealthCheckConfig<CustomConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the health check, vs the health check being enabled/disabled by default.
    // If this is `None`, the value will match the value of `HealthCheck#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub enable: Option<bool>,
}

impl CommonConfig {
    pub fn enabled<S>(&self, context: &AppContext<S>) -> bool {
        self.enable
            .unwrap_or(context.config().health_check.default_enable)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct HealthCheckConfig<T> {
    #[serde(flatten)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
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
        config.health_check.default_enable = default_enable;

        let context = AppContext::<()>::test(Some(config), None, None).unwrap();

        let common_config = CommonConfig { enable };

        // Act/Assert
        assert_eq!(common_config.enabled(&context), expected_enabled);
    }
}
