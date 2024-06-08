mod common;
#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "http")]
pub mod http;
pub mod worker;

use crate::app::context::AppContext;
#[cfg(feature = "grpc")]
use crate::config::service::grpc::GrpcServiceConfig;
#[cfg(feature = "http")]
use crate::config::service::http::HttpServiceConfig;
#[cfg(feature = "sidekiq")]
use crate::config::service::worker::sidekiq::SidekiqServiceConfig;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Service {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[cfg(feature = "http")]
    #[validate(nested)]
    pub http: ServiceConfig<HttpServiceConfig>,

    #[cfg(feature = "grpc")]
    #[validate(nested)]
    pub grpc: ServiceConfig<GrpcServiceConfig>,

    #[cfg(feature = "sidekiq")]
    #[validate(nested)]
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

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ServiceConfig<T: Validate> {
    #[serde(flatten, default)]
    pub common: CommonConfig,
    #[serde(flatten)]
    #[validate(nested)]
    pub custom: T,
}
