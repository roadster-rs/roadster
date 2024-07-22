use crate::service::worker::sidekiq::app_worker::AppWorkerConfig;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};
use url::Url;
use validator::Validate;

pub fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SidekiqServiceConfig {
    /// The number of Sidekiq workers that can run at the same time. Adjust as needed based on
    /// your workload and resource (cpu/memory/etc) usage.
    ///
    /// If your workload is largely CPU-bound (computationally expensive), this should probably
    /// match your CPU count. This is the default if not provided.
    ///
    /// If your workload is largely IO-bound (e.g. reading from a DB, making web requests and
    /// waiting for responses, etc), this can probably be quite a bit higher than your CPU count.
    #[serde(default = "SidekiqServiceConfig::default_num_workers")]
    pub num_workers: u32,

    /// The names of the worker queues to handle.
    // Todo: Allow overriding this via CLI args?
    #[serde(default)]
    pub queues: Vec<String>,

    #[validate(nested)]
    pub redis: Redis,

    #[serde(default)]
    #[validate(nested)]
    pub periodic: Periodic,

    /// The default app worker config. Values can be overridden on a per-worker basis by
    /// implementing the corresponding [crate::service::worker::sidekiq::app_worker::AppWorker] methods.
    #[serde(default)]
    #[validate(nested)]
    pub app_worker: AppWorkerConfig,
}

impl SidekiqServiceConfig {
    fn default_num_workers() -> u32 {
        num_cpus::get() as u32
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
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
#[non_exhaustive]
pub enum StaleCleanUpBehavior {
    /// Do not automatically remove periodic jobs.
    Manual,
    /// Automatically remove all periodic jobs that were registered previously. The jobs will
    /// be removed before any new jobs are registered.
    AutoCleanAll,
    /// Automatically remove periodic jobs that were registered previously, but were not registered
    /// during start up of the current app instance.
    AutoCleanStale,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Redis {
    pub uri: Url,
    /// The configuration for the Redis connection pool used for enqueuing Sidekiq jobs in Redis.
    #[serde(default)]
    #[validate(nested)]
    pub enqueue_pool: ConnectionPool,
    /// The configuration for the Redis connection pool used by [sidekiq::Processor] to fetch
    /// Sidekiq jobs from Redis.
    #[serde(default)]
    #[validate(nested)]
    pub fetch_pool: ConnectionPool,
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ConnectionPool {
    pub min_idle: Option<u32>,
    pub max_connections: Option<u32>,
}

#[cfg(test)]
mod deserialize_tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::assert_toml_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(
        r#"
        # The default `num-workers` is the same as the number of cpu cores, so we always set
        # this in our tests so they always pass regardless of the host's hardware.
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        queues = ["foo"]
        [redis]
        uri = "redis://localhost:6379"
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        [redis.enqueue-pool]
        min-idle = 1
        [redis.fetch-pool]
        min-idle = 2
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        [redis.enqueue-pool]
        max-connections = 1
        [redis.fetch-pool]
        max-connections = 2
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        [periodic]
        stale-cleanup = "auto-clean-stale"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sidekiq(_case: TestCase, #[case] config: &str) {
        let sidekiq: SidekiqServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(sidekiq);
    }
}
