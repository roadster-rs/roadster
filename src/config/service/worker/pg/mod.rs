use crate::config::database::{DbPoolConfig, StatementLogConfig};
use crate::config::service::worker::StaleCleanUpBehavior;
use crate::util::serde::default_true;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::time::Duration;
use url::Url;
use validator::Validate;

#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct PgWorkerServiceConfig {
    #[serde(default)]
    pub database: Option<DbConfig>,

    #[serde(default)]
    pub queue_fetch_config: Option<QueueFetchConfig>,

    #[serde(default)]
    #[validate(nested)]
    pub periodic: Periodic,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct DbConfig {
    /// The URI of the postgres BD to use for the PG worker service. If not provided, will use the
    /// URI from the main database config.
    #[serde(default)]
    pub uri: Option<Url>,

    /// Create a temporary database in the same DB host from the `uri` field.
    #[serde(default)]
    pub temporary_test_db: bool,

    /// Automatically clean up (drop) the temporary test DB that was created by setting
    /// `temporary_test_db` to `true`. Note that the test DB will only be cleaned up if the closure
    /// passed to [`crate::app::run_test`] or [`crate::app::run_test_with_result`] doesn't panic.
    #[cfg(feature = "testing")]
    #[serde(default = "default_true")]
    pub temporary_test_db_clean_up: bool,

    /// Configuration for the DB pool. If not provided, will re-use the configuration from
    /// [`crate::config::database::Database`], including the DB URI. If not provided and the
    /// `db-sea-orm` feature is enabled, the underlying [`sqlx::Pool`] from `sea-orm` will be
    /// used.
    #[validate(nested)]
    #[serde(flatten, default)]
    pub pool_config: Option<DbPoolConfig>,

    #[validate(nested)]
    #[serde(default, flatten)]
    pub statement_log_config: Option<StatementLogConfig>,
}

#[derive(Default, Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Periodic {
    /// Enable or disable the periodic worker task that polls the periodic job queue and
    /// enqueues jobs as they become available from the periodic queue.
    #[serde(default = "default_true")]
    pub enable: bool,

    #[serde(default)]
    pub stale_cleanup: StaleCleanUpBehavior,
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

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct QueueFetchConfig {
    /// How long to wait before fetching from a queue again when the previous fetch
    /// experienced an error (e.g., db timeout).
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub error_delay: Option<Duration>,

    /// How long to wait before fetching from a queue that was empty on a previous fetch.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub empty_delay: Option<Duration>,
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
        [database]
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        [database]
        uri = "postgres://localhost:5432/example"
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        [database]
        uri = "postgres://localhost:5432/example"
        max-connections = 1
        connect-timeout = 2000
        "#
    )]
    #[case(
        r#"
        [database]
        connect-timeout = 2000
        connect-lazy = true
        acquire-timeout = 5000
        idle-timeout = 10000
        max-lifetime = 60000
        min-connections = 5
        max-connections = 6
        test-on-checkout = true
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn worker_pg(_case: TestCase, #[case] config: &str) {
        let worker_pg: PgWorkerServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(worker_pg);
    }
}
