use crate::app::context::AppContext;
use crate::util::serde_util::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
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
