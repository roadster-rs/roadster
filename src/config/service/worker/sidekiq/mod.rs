use crate::service::worker::sidekiq::app_worker::AppWorkerConfig;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

    /// The strategy for balancing the priority of fetching queues' jobs from Redis. Defaults
    /// to [`BalanceStrategy::RoundRobin`].
    ///
    /// The Redis API used to fetch jobs ([brpop](https://redis.io/docs/latest/commands/brpop/))
    /// checks queues for jobs in the order the queues are provided. This means that if the first
    /// queue in the list provided to [`sidekiq::Processor::new`] always has an item, the other queues
    /// will never have their jobs run. To mitigate this, a [`BalanceStrategy`] is provided
    /// (configurable in this field) to ensure that no queue is starved indefinitely.
    #[serde(default)]
    pub balance_strategy: BalanceStrategy,

    /// The names of the worker queues to handle.
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

    /// Queue-specific configurations. The queues specified in this field do not need to match
    /// the list of queues listed in the `queues` field.
    #[serde(default)]
    #[validate(nested)]
    pub queue_config: BTreeMap<String, QueueConfig>,
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

    /// Options for creating a Test Container instance for the DB. If enabled, the `Redis#uri`
    /// field will be overridden to be the URI for the Test Container instance that's created when
    /// building the app's [`crate::app::context::AppContext`].
    #[cfg(feature = "test-containers")]
    #[serde(default)]
    pub test_container: Option<crate::config::TestContainer>,
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct ConnectionPool {
    pub min_idle: Option<u32>,
    pub max_connections: Option<u32>,
}

#[derive(
    Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum BalanceStrategy {
    /// Rotate the list of queues by 1 every time jobs are fetched from Redis. This allows each
    /// queue in the list to have an equal opportunity to have its jobs run.
    #[default]
    RoundRobin,
    /// Do not modify the list of queues. Warning: This can lead to queue starvation! For example,
    /// if the first queue in the list provided to [`sidekiq::Processor::new`] is heavily used and always
    /// has a job available to run, then the jobs in the other queues will never be run.
    None,
}

impl From<BalanceStrategy> for sidekiq::BalanceStrategy {
    fn from(value: BalanceStrategy) -> Self {
        match value {
            BalanceStrategy::RoundRobin => sidekiq::BalanceStrategy::RoundRobin,
            BalanceStrategy::None => sidekiq::BalanceStrategy::None,
        }
    }
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct QueueConfig {
    /// Similar to `SidekiqServiceConfig#num_workers`, except allows configuring the number of
    /// additional workers to dedicate to a specific queue. If provided, `num_workers` additional
    /// workers will be created for this specific queue.
    pub num_workers: Option<u32>,
}

impl From<&QueueConfig> for sidekiq::QueueConfig {
    fn from(value: &QueueConfig) -> Self {
        value
            .num_workers
            .iter()
            .fold(Default::default(), |config, num_workers| {
                config.num_workers(*num_workers as usize)
            })
    }
}

impl From<QueueConfig> for sidekiq::QueueConfig {
    fn from(value: QueueConfig) -> Self {
        sidekiq::QueueConfig::from(&value)
    }
}

#[cfg(test)]
mod deserialize_tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use ::sidekiq::BalanceStrategy as SidekiqBalanceStrategy;
    use ::sidekiq::QueueConfig as SidekiqQueueConfig;
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
    #[case(
        r#"
        num-workers = 1
        balance-strategy = "none"
        [redis]
        uri = "redis://localhost:6379"
        [periodic]
        stale-cleanup = "auto-clean-stale"
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        balance-strategy = "round-robin"
        [redis]
        uri = "redis://localhost:6379"
        [periodic]
        stale-cleanup = "auto-clean-stale"
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        [periodic]
        stale-cleanup = "auto-clean-stale"
        [queue-config]
        "foo" = { num-workers = 10 }
        [queue-config.bar]
        num-workers = 100
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [redis]
        uri = "redis://localhost:6379"
        [app-worker]
        max-retries = 10
        timeout = true
        max-duration = 100
        disable-argument-coercion = true
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sidekiq(_case: TestCase, #[case] config: &str) {
        let sidekiq: SidekiqServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(sidekiq);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_num_workers() {
        assert_eq!(
            SidekiqServiceConfig::default_num_workers(),
            num_cpus::get() as u32
        );
    }

    #[rstest]
    #[case(BalanceStrategy::RoundRobin)]
    #[case(BalanceStrategy::None)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn balance_strat_to_sidekiq_balance_strat(#[case] strategy: BalanceStrategy) {
        let sidekiq_strategy: SidekiqBalanceStrategy = strategy.clone().into();
        match sidekiq_strategy {
            SidekiqBalanceStrategy::RoundRobin => {
                assert!(matches!(strategy, BalanceStrategy::RoundRobin))
            }
            SidekiqBalanceStrategy::None => {
                assert!(matches!(strategy, BalanceStrategy::None))
            }
            _ => unimplemented!(),
        }
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn queue_config_to_sidekiq_queue_config() {
        let num_workers = 10;
        let config = QueueConfig {
            num_workers: Some(num_workers),
        };
        let sidekiq_config: SidekiqQueueConfig = config.into();
        assert_eq!(sidekiq_config.num_workers, num_workers as usize);
    }
}
