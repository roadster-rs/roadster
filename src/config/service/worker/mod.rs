use crate::config::service::ServiceConfig;
use crate::config::service::worker::pg::WorkerPgServiceConfig;
use crate::config::service::worker::sidekiq::SidekiqServiceConfig;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum_macros::{EnumString, IntoStaticStr};
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
    pub enqueue_config: crate::worker::EnqueueConfig,

    /// The default worker config if not overridden by a worker.
    #[validate(nested)]
    #[serde(default)]
    pub worker_config: crate::worker::WorkerConfig,

    /// Worker configurations specific to sidekiq-backed queues.
    #[cfg(feature = "worker-sidekiq")]
    #[validate(nested)]
    pub sidekiq: ServiceConfig<WorkerConfig<SidekiqServiceConfig>>,

    /// Worker configurations specific to postgres-backed (`pgmq`) queues.
    #[cfg(feature = "worker-pg")]
    #[validate(nested)]
    pub pg: ServiceConfig<WorkerConfig<WorkerPgServiceConfig>>,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig<T: Validate> {
    #[serde(flatten, default)]
    #[validate(nested)]
    pub common: CommonConfig,
    #[serde(flatten)]
    #[validate(nested)]
    pub custom: T,
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CommonConfig {
    /// Default [`crate::worker::EnqueueConfig`] to use for [`crate::worker::Worker`]s that don't
    /// provide values to override the defaults defined here.
    #[serde(default)]
    pub enqueue_config: crate::worker::EnqueueConfig,

    /// Default [`crate::worker::WorkerConfig`] to use for [`crate::worker::Worker`]s that don't
    /// provide values to override the defaults defined here.
    #[serde(default)]
    pub worker_config: crate::worker::WorkerConfig,

    /// The number of workers that can run at the same time. Adjust as needed based on
    /// your workload and resource (cpu/memory/etc) usage.
    ///
    /// If your workload is largely CPU-bound (computationally expensive), this should probably
    /// match your CPU count. This is the default if not provided.
    ///
    /// If your workload is largely IO-bound (e.g. reading from a DB, making web requests and
    /// waiting for responses, etc), this can probably be quite a bit higher than your CPU count.
    #[serde(default = "CommonConfig::default_num_workers")]
    pub num_workers: u32,

    /// The strategy for balancing the priority of fetching queues' jobs. Defaults
    /// to [`BalanceStrategy::RoundRobin`].
    #[serde(default)]
    pub balance_strategy: BalanceStrategy,

    /// The names of the worker queues to handle.
    #[serde(default)]
    pub queues: Vec<String>,

    /// Queue-specific configurations. The queues specified in this field do not need to match
    /// the list of queues listed in the `queues` field.
    #[serde(default)]
    #[validate(nested)]
    pub queue_config: BTreeMap<String, QueueConfig>,
}

impl CommonConfig {
    fn default_num_workers() -> u32 {
        num_cpus::get() as u32
    }
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum BalanceStrategy {
    /// Rotate the list of queues by 1 every time jobs are fetched. This allows each
    /// queue in the list to have an equal opportunity to have its jobs run.
    #[default]
    RoundRobin,
    /// Do not modify the list of queues. Warning: This can lead to queue starvation! For example,
    /// if the first queue in the list is heavily used and always has a job available to run,
    /// then the jobs in the other queues will never be run.
    None,
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct QueueConfig {
    /// Similar to `CommonConfig#num_workers`, except allows configuring the number of
    /// additional workers to dedicate to a specific queue. If provided, `num_workers` additional
    /// workers will be created for this specific queue.
    pub num_workers: Option<u32>,
}
