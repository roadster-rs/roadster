pub mod http;
pub mod worker;

#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::service::http::HttpServiceConfig;
#[cfg(feature = "sidekiq")]
use crate::config::service::worker::sidekiq::SidekiqServiceConfig;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Service {
    #[serde(default = "default_true")]
    pub default_enable: bool,
    pub http: ServiceConfig<HttpServiceConfig>,
    #[cfg(feature = "sidekiq")]
    pub sidekiq: ServiceConfig<SidekiqServiceConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the service, vs the service being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Middleware#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
}

impl CommonConfig {
    pub fn enabled<T>(&self, context: &AppContext<T>) -> bool {
        self.enable
            .unwrap_or(context.config().service.default_enable)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ServiceConfig<T> {
    #[serde(flatten, default)]
    pub common: CommonConfig,
    #[serde(flatten)]
    pub custom: T,
}
