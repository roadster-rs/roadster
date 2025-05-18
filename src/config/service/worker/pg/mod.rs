use crate::config::database::Database;
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::collections::BTreeMap;
use std::time::Duration;
use url::Url;
use validator::Validate;

pub(crate) fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerPgServiceConfig {
    /// The number of background workers that can run at the same time. Adjust as needed based on
    /// your workload and resource (cpu/memory/etc) usage.
    ///
    /// If your workload is largely CPU-bound (computationally expensive), this should probably
    /// match your CPU count. This is the default if not provided.
    ///
    /// If your workload is largely IO-bound (e.g. reading from a DB, making web requests and
    /// waiting for responses, etc), this can probably be quite a bit higher than your CPU count.
    #[serde(default = "WorkerPgServiceConfig::default_num_workers")]
    pub num_workers: u32,

    /// The names of the worker queues to handle. If not provided, handle all the queues that are
    /// registered in the PG worker service. If a list is provided, only the queues specified in the
    /// list will be handled, even if other worker queues are registered with the PG worker service.
    /// Note that an empty list will result in no queues being handled.
    ///
    /// Queues can also be specified in the `queue_config` map.
    #[serde(default)]
    pub queues: Option<Vec<String>>,

    /// Configuration for the DB pool. If not provided, will re-use the configuration from
    /// [`crate::config::database::Database`], including the DB URI. If not provided and the
    /// `db-sea-orm` feature is enabled, the underlying [`sqlx::Pool`] from `sea-orm` will be
    /// used.
    #[validate(nested)]
    pub db_pool: Option<DbPoolConfig>,

    /// The default app worker config. Values can be overridden on a per-worker basis by
    /// implementing the corresponding methods.
    #[serde(default)]
    #[validate(nested)]
    pub worker_config: WorkerConfig,

    /// Queue-specific configurations. The queues specified in this field do not need to match
    /// the list of queues listed in the `queues` field.
    #[serde(default)]
    #[validate(nested)]
    pub queue_config: Option<BTreeMap<String, QueueConfig>>,
}

impl WorkerPgServiceConfig {
    fn default_num_workers() -> u32 {
        num_cpus::get() as u32
    }
}

#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct QueueConfig {
    /// Similar to `WorkerPgServiceConfig#num_workers`, except allows configuring the number of
    /// additional workers to dedicate to a specific queue. If provided, `num_workers` additional
    /// workers will be created for this specific queue.
    pub num_workers: Option<u32>,
}

/// Action to take when a job completes processing, either by being processed successfully, or by
/// running out of retry attempts.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CompletedAction {
    /// Move the message to the queue's archive table.
    Archive,
    /// Delete the message.
    Delete,
}

// Todo: consolidate with `service::worker::sidekiq::app_worker::AppWorkerConfig`?
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Validate, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig {
    /// The maximum number of times a job should be retried on failure.
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// True if Roadster should enforce a timeout on the app's workers. The default duration of
    /// the timeout can be configured with the `max-duration` option.
    #[serde(default)]
    pub timeout: Option<bool>,

    /// The maximum duration workers should run for. The timeout is only enforced if `timeout`
    /// is `true`.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub max_duration: Option<Duration>,

    /// The action to take when a job in the queue completes successfully.
    #[serde(default)]
    pub success_action: Option<CompletedAction>,

    /// The action to take when a job in the queue fails and has no more retry attempts.
    #[serde(default)]
    pub failure_action: Option<CompletedAction>,
}

// Todo: consolidate with the sea-orm connection options?
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct DbPoolConfig {
    /// The URI of the postgres BD to use for the PG worker service. If not provided, will use the
    /// URI from the main database config.
    #[serde(default)]
    pub uri: Option<Url>,

    #[serde(default = "Database::default_connect_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub connect_timeout: Duration,

    /// Whether to attempt to connect to the DB immediately when the DB connection pool is created.
    /// If `true` will wait to connect to the DB until the first DB query is attempted.
    #[serde(default = "default_true")]
    pub connect_lazy: bool,

    #[serde(default = "Database::default_acquire_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub acquire_timeout: Duration,

    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub idle_timeout: Option<Duration>,

    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub max_lifetime: Option<Duration>,

    #[serde(default)]
    pub min_connections: u32,

    pub max_connections: u32,

    #[serde(default = "default_true")]
    pub test_on_checkout: bool,
}

impl From<DbPoolConfig> for sqlx::pool::PoolOptions<sqlx::Postgres> {
    fn from(value: DbPoolConfig) -> Self {
        Self::from(&value)
    }
}

impl From<&DbPoolConfig> for sqlx::pool::PoolOptions<sqlx::Postgres> {
    fn from(value: &DbPoolConfig) -> Self {
        sqlx::pool::PoolOptions::new()
            .acquire_timeout(value.acquire_timeout)
            .idle_timeout(value.idle_timeout)
            .max_lifetime(value.max_lifetime)
            .min_connections(value.min_connections)
            .max_connections(value.max_connections)
            .test_before_acquire(value.test_on_checkout)
    }
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
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [db-pool]
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [db-pool]
        uri = "redis://localhost:6379"
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        queues = ["foo"]
        [db-pool]
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        queues = ["foo"]
        [db-pool]
        uri = "postgres://localhost:5432/example"
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        queues = ["foo"]
        [db-pool]
        connect-timeout = 1
        connect-lazy = true
        acquire-timeout = 2
        idle-timeout = 3
        max-lifetime = 4
        min-connections = 5
        max-connections = 6
        test-on-checkout = true
        "#
    )]
    #[case(
        r#"
        num-workers = 1
        [db-pool]
        max-connections = 1
        [worker-config]
        max-retries = 25
        timeout = true
        max-duration = 60
        success-action = "delete"
        failure-action = "archive"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn worker_pg(_case: TestCase, #[case] config: &str) {
        let worker_pg: WorkerPgServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(worker_pg);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_num_workers() {
        assert_eq!(
            WorkerPgServiceConfig::default_num_workers(),
            num_cpus::get() as u32
        );
    }
}
