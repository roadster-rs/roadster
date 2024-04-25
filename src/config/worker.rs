use crate::worker::app_worker::AppWorkerConfig;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Worker {
    // Todo: Make Redis optional for workers?
    pub sidekiq: Sidekiq,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Sidekiq {
    // Todo: Make Redis optional for workers?
    pub redis: Redis,

    /// The number of Sidekiq workers that can run at the same time. Adjust as needed based on
    /// your workload and resource (cpu/memory/etc) usage.
    ///
    /// If your workload is largely CPU-bound (computationally expensive), this should probably
    /// match your CPU count. This is the default if not provided.
    ///
    /// If your workload is largely IO-bound (e.g. reading from a DB, making web requests and
    /// waiting for responses, etc), this can probably be quite a bit higher than your CPU count.
    #[serde(default = "Sidekiq::default_num_workers")]
    pub num_workers: u32,

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

impl Sidekiq {
    fn default_num_workers() -> u32 {
        num_cpus::get() as u32
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Redis {
    pub uri: Url,
    /// The configuration for the Redis connection pool used for enqueuing Sidekiq jobs in Redis.
    #[serde(default)]
    pub enqueue_pool: ConnectionPool,
    /// The configuration for the Redis connection pool used by [sidekiq::Processor] to fetch
    /// Sidekiq jobs from Redis.
    #[serde(default)]
    pub fetch_pool: ConnectionPool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConnectionPool {
    pub min_idle: Option<u32>,
    pub max_connections: Option<u32>,
}
