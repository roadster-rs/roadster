use crate::app::context::AppContext;
use crate::config::CustomConfig;
use crate::util::serde::default_true;
use axum_core::extract::FromRef;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::collections::BTreeMap;
use std::time::Duration;
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

    #[validate(nested)]
    pub max_duration: MaxDuration,

    #[cfg(feature = "db-sql")]
    #[validate(nested)]
    pub database: HealthCheckConfig<crate::config::EmptyConfig>,

    #[cfg(feature = "sidekiq")]
    #[validate(nested)]
    pub sidekiq: HealthCheckConfig<crate::config::EmptyConfig>,

    #[cfg(feature = "email-smtp")]
    #[validate(nested)]
    pub smtp: HealthCheckConfig<crate::config::EmptyConfig>,

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
    #[validate(nested)]
    pub custom: BTreeMap<String, HealthCheckConfig<CustomConfig>>,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
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
    pub fn enabled<S>(&self, state: &S) -> bool
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        self.enable.unwrap_or(
            AppContext::from_ref(state)
                .config()
                .health_check
                .default_enable,
        )
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct HealthCheckConfig<T: Validate> {
    #[serde(flatten)]
    #[validate(nested)]
    pub common: CommonConfig,
    #[serde(flatten)]
    #[validate(nested)]
    pub custom: T,
}

/// The maximum duration to wait for health checks to succeed before timing out and assuming
/// the checks failed.
#[serde_as]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct MaxDuration {
    /// The maximum time (in milliseconds) to wait  when running checks on app startup.
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub startup: Duration,
    /// The maximum time (in milliseconds) to wait when running checks via a web API (HTTP, gRPC, etc).
    /// In the default `_health` HTTP endpoint, this can be overridden via the `maxDuration`
    /// query parameter.
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub api: Duration,
    /// The maximum time (in milliseconds) to wait when running checks via the CLI. This can be
    /// overridden via the `-d/--max-duration` CLI arg.
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub cli: Duration,
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
        config.health_check.default_enable = default_enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let common_config = CommonConfig { enable };

        // Act/Assert
        assert_eq!(common_config.enabled(&context), expected_enabled);
    }
}
