#[cfg(feature = "worker-pg")]
use crate::config::service::worker::pg::PgWorkerServiceConfig;
#[cfg(feature = "worker-sidekiq")]
use crate::config::service::worker::sidekiq::SidekiqWorkerServiceConfig;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::collections::{BTreeMap, BTreeSet};
use strum_macros::{EnumString, IntoStaticStr};
use validator::Validate;

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

pub(crate) fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerServiceConfig {
    /// The default enqueue config if not overridden by a worker.
    #[validate(nested)]
    #[serde(default)]
    pub enqueue_config: crate::worker::config::EnqueueConfig,

    /// The default worker config if not overridden by a worker.
    #[validate(nested)]
    #[serde(default)]
    pub worker_config: crate::worker::config::WorkerConfig,

    /// Worker configurations specific to sidekiq-backed queues.
    #[cfg(feature = "worker-sidekiq")]
    #[validate(nested)]
    pub sidekiq: crate::config::service::ServiceConfig<WorkerConfig<SidekiqWorkerServiceConfig>>,

    /// Worker configurations specific to postgres-backed (`pgmq`) queues.
    #[cfg(feature = "worker-pg")]
    #[validate(nested)]
    pub pg: crate::config::service::ServiceConfig<WorkerConfig<PgWorkerServiceConfig>>,
}

#[serde_with::skip_serializing_none]
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

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CommonConfig {
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

    /// The names of the worker queues to handle in a shared pool of worker tasks.
    ///
    /// If not provided, will default to all of the queues for all registered
    /// [`crate::worker::Worker`]s (minus any queues specified in the `queue_config` field).
    #[serde(default)]
    pub queues: Option<BTreeSet<String>>,

    /// Queue-specific configurations. The queues specified in this field will be processed in
    /// dedicated worker tasks and removed from the shared pool. The queues specified in this field
    /// do not need to match the list of queues listed in the `queues` field.
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

#[serde_with::skip_serializing_none]
#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct QueueConfig {
    /// Similar to `CommonConfig#num_workers`, except allows configuring the number of
    /// additional workers to dedicate to a specific queue. If provided, `num_workers` additional
    /// workers will be created for this specific queue.
    pub num_workers: Option<u32>,
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr,
)]
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
    #[default]
    AutoCleanStale,
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_num_workers() {
        assert_eq!(
            super::CommonConfig::default_num_workers(),
            num_cpus::get() as u32
        );
    }
}

// To simplify testing, these are only run when all of the config fields are available
#[cfg(all(
    test,
    feature = "worker-sidekiq",
    feature = "worker-pg",
    feature = "db-diesel-pool-async"
))]
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
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        [enqueue-config]
        queue = "default"
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        [worker-config]
        timeout = true
        max-duration = 120000
        [worker-config.retry-config]
        max-retries = 10
        delay = 10000
        delay-offset = 20000
        max-delay = 30000
        backoff-strategy = "exponential"
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        [worker-config.pg]
        success-action = "archive"
        failure-action = "delete"
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        [worker-config.sidekiq]
        disable-argument-coercion = true
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [sidekiq.redis.enqueue-pool]
        min-idle = 1
        max-connections = 2
        [sidekiq.redis.fetch-pool]
        min-idle = 3
        max-connections = 4
        [sidekiq.periodic]
        stale-cleanup = "auto-clean-all"
        [pg]
        num-workers = 8
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        [pg.database]
        uri = "postgres://postgres:postgres@localhost:5432/example_dev"
        connect-timeout = 1000
        connect-lazy = false
        acquire-timeout = 2000
        idle-timeout = 10000
        max-lifetime = 60000
        min-connections = 1
        max-connections = 2
        test-on-checkout = false
        retry-connection = false
        "#
    )]
    #[case(
        r#"
        [sidekiq]
        num-workers = 8
        [sidekiq.redis]
        uri = "redis://localhost:6379"
        [pg]
        num-workers = 8
        [pg.queue-fetch-config]
        error-delay = 15000
        empty-delay = 20000
        [pg.periodic]
        enable = false
        stale-cleanup = "auto-clean-all"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_tests(_case: TestCase, #[case] config: &str) {
        let worker_service_config: WorkerServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(worker_service_config);
    }
}
