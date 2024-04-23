use crate::config::app_config::Redis;
use crate::worker::app_worker::AppWorkerConfig;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};

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

    /// The names of the worker queues to handle.
    // Todo: Allow overriding this via CLI args?
    #[serde(default)]
    pub queues: Vec<String>,

    #[serde(default)]
    pub periodic: Periodic,

    /// The default app worker config. Values can be overridden on a per-worker basis by
    /// implementing the corresponding [crate::worker::app_worker::AppWorker] methods.
    #[serde(default, flatten)]
    pub worker_config: AppWorkerConfig,
}

#[cfg(feature = "sidekiq")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Periodic {
    pub stale_cleanup: StaleCleanUpBehavior,
}

impl Default for Periodic {
    fn default() -> Self {
        Self {
            stale_cleanup: StaleCleanUpBehavior::AutoCleanStale,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum StaleCleanUpBehavior {
    Manual,
    AutoCleanAll,
    AutoCleanStale,
}
