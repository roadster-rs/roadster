use crate::config::app_config::Redis;
use crate::worker::AppWorkerConfig;
use serde_derive::{Deserialize, Serialize};

#[cfg(feature = "sidekiq")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Worker {
    // Todo: Make Redis optional for workers?
    #[cfg(feature = "sidekiq")]
    pub sidekiq: Sidekiq,
}

#[cfg(feature = "sidekiq")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Sidekiq {
    // Todo: Make Redis optional for workers?
    pub redis: Redis,

    #[serde(default)]
    pub queue_names: Vec<String>,

    /// The default worker config.
    #[serde(default)]
    pub worker_config: AppWorkerConfig,
}