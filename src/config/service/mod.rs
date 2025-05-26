mod common;
#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "worker")]
pub mod worker;

use crate::app::context::AppContext;
use crate::config::CustomConfig;
#[cfg(feature = "grpc")]
use crate::config::service::grpc::GrpcServiceConfig;
#[cfg(feature = "http")]
use crate::config::service::http::HttpServiceConfig;
#[cfg(feature = "worker-pg")]
use crate::config::service::worker::pg::WorkerPgServiceConfig;
#[cfg(feature = "sidekiq")]
use crate::config::service::worker::sidekiq::SidekiqServiceConfig;
use crate::util::serde::default_true;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
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

    #[cfg(feature = "worker-pg")]
    #[validate(nested)]
    pub worker_pg: ServiceConfig<WorkerPgServiceConfig>,

    #[serde(flatten)]
    #[validate(nested)]
    pub custom: BTreeMap<String, ServiceConfig<CustomConfig>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Validate)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct CommonConfig {
    // Optional so we can tell the difference between a consumer explicitly enabling/disabling
    // the service, vs the service being enabled/disabled by default.
    // If this is `None`, the value will match the value of `Middleware#default_enable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
}

impl CommonConfig {
    pub fn enabled(&self, context: &AppContext) -> bool {
        self.enable
            .unwrap_or(context.config().service.default_enable)
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ServiceConfig<T: Validate> {
    #[serde(flatten, default)]
    #[validate(nested)]
    pub common: CommonConfig,
    #[serde(flatten)]
    #[validate(nested)]
    pub custom: T,
}
