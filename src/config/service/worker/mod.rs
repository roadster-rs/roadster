use crate::config::service::ServiceConfig;
use crate::config::service::worker::pg::WorkerPgServiceConfig;
use crate::config::service::worker::sidekiq::SidekiqServiceConfig;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerServiceConfig {
    /// The default enqueue config if not overridden by a worker.
    #[validate(nested)]
    #[serde(default)]
    pub enqueue_config: crate::service::worker::EnqueueConfig,

    /// The default worker config if not overridden by a worker.
    #[validate(nested)]
    #[serde(default)]
    pub worker_config: crate::service::worker::WorkerConfig,

    /// Worker configurations specific to sidekiq-backed queues.
    #[cfg(feature = "worker-sidekiq")]
    #[validate(nested)]
    pub sidekiq: ServiceConfig<SidekiqServiceConfig>,

    /// Worker configurations specific to postgres-backed (pgmq) queues.
    #[cfg(feature = "worker-pg")]
    #[validate(nested)]
    pub worker_pg: ServiceConfig<WorkerPgServiceConfig>,
}
