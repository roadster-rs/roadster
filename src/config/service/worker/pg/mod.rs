use crate::config::database::Database;
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
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
    /// Configuration for the DB pool. If not provided, will re-use the configuration from
    /// [`crate::config::database::Database`], including the DB URI. If not provided and the
    /// `db-sea-orm` feature is enabled, the underlying [`sqlx::Pool`] from `sea-orm` will be
    /// used.
    #[validate(nested)]
    pub db_pool: Option<DbPoolConfig>,
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
}
