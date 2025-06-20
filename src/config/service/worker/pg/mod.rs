use crate::config::database::{Database, DbPoolConfig};
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::time::Duration;
use url::Url;
use validator::Validate;

#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerPgServiceConfig {
    #[serde(default)]
    pub db_config: Option<DbConfig>,
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

    /// Configuration for the DB pool. If not provided, will re-use the configuration from
    /// [`crate::config::database::Database`], including the DB URI. If not provided and the
    /// `db-sea-orm` feature is enabled, the underlying [`sqlx::Pool`] from `sea-orm` will be
    /// used.
    #[validate(nested)]
    #[serde(flatten, default)]
    pub pool_config: Option<DbPoolConfig>,
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
        [db-config]
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        [db-config]
        uri = "redis://localhost:6379"
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        [db-config]
        uri = "postgres://localhost:5432/example"
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        [db-config]
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn worker_pg(_case: TestCase, #[case] config: &str) {
        let worker_pg: WorkerPgServiceConfig = toml::from_str(config).unwrap();

        assert_toml_snapshot!(worker_pg);
    }
}
