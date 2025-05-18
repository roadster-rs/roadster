use crate::util::serde::default_true;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Duration;
use url::Url;
use validator::Validate;

#[serde_as]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Database {
    /// This can be overridden with an environment variable, e.g. `ROADSTER__DATABASE__URI=postgres://example:example@example:1234/example_app`
    pub uri: Url,

    /// Whether to automatically apply migrations during the app's start up. Migrations can also
    /// be manually performed via the `roadster migration [COMMAND]` CLI command.
    pub auto_migrate: bool,

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

    /// See [`bb8_8::Builder::retry_connection`]
    #[cfg(feature = "db-diesel-pool-async")]
    #[serde(default = "default_true")]
    pub retry_connection: bool,

    /// Create a temporary database in the same DB host from the `uri` field.
    #[serde(default)]
    pub temporary_test_db: bool,

    /// Automatically clean up (drop) the temporary test DB that was created by setting
    /// `temporary_test_db` to `true`. Note that the test DB will only be cleaned up if the closure
    /// passed to [`crate::app::run_test`] or [`crate::app::run_test_with_result`] doesn't panic.
    #[cfg(feature = "testing")]
    #[serde(default = "default_true")]
    pub temporary_test_db_clean_up: bool,

    /// Options for creating a Test Container instance for the DB. If enabled, the `Database#uri`
    /// field will be overridden to be the URI for the Test Container instance that's created when
    /// building the app's [`crate::app::context::AppContext`].
    #[cfg(feature = "test-containers")]
    #[serde(default)]
    #[validate(nested)]
    pub test_container: Option<crate::config::TestContainer>,
}

impl Database {
    pub(crate) fn default_connect_timeout() -> Duration {
        Duration::from_millis(1000)
    }

    pub(crate) fn default_acquire_timeout() -> Duration {
        Duration::from_millis(1000)
    }
}

#[cfg(feature = "db-sea-orm")]
impl From<Database> for sea_orm::ConnectOptions {
    fn from(database: Database) -> Self {
        sea_orm::ConnectOptions::from(&database)
    }
}

#[cfg(feature = "db-sea-orm")]
impl From<&Database> for sea_orm::ConnectOptions {
    fn from(database: &Database) -> Self {
        let mut options = sea_orm::ConnectOptions::new(database.uri.to_string());
        options
            .test_before_acquire(database.test_on_checkout)
            .connect_timeout(database.connect_timeout)
            .connect_lazy(database.connect_lazy)
            .acquire_timeout(database.acquire_timeout)
            .min_connections(database.min_connections)
            .max_connections(database.max_connections)
            .sqlx_logging(false);
        if let Some(idle_timeout) = database.idle_timeout {
            options.idle_timeout(idle_timeout);
        }
        if let Some(max_lifetime) = database.max_lifetime {
            options.max_lifetime(max_lifetime);
        }
        options
    }
}

#[cfg(feature = "worker-pg")]
impl From<Database> for sqlx::pool::PoolOptions<sqlx::Postgres> {
    fn from(value: Database) -> Self {
        Self::from(&value)
    }
}

#[cfg(feature = "worker-pg")]
impl From<&Database> for sqlx::pool::PoolOptions<sqlx::Postgres> {
    fn from(value: &Database) -> Self {
        sqlx::pool::PoolOptions::new()
            .test_before_acquire(value.test_on_checkout)
            .acquire_timeout(value.acquire_timeout)
            .min_connections(value.min_connections)
            .max_connections(value.max_connections)
            .idle_timeout(value.idle_timeout)
            .max_lifetime(value.max_lifetime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use rstest::fixture;

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest::rstest]
    #[case(
        r#"
        uri = "https://example.com:1234"
        auto-migrate = true
        max-connections = 1
        "#
    )]
    #[case(
        r#"
        uri = "https://example.com:1234"
        auto-migrate = true
        max-connections = 1
        connect-timeout = 1000
        acquire-timeout = 2000
        idle-timeout = 3000
        max-lifetime = 4000
        "#
    )]
    #[cfg(feature = "db-diesel-pool-async")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let database: Database = toml::from_str(config).unwrap();

        insta::assert_toml_snapshot!(database);
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn db_config() -> Database {
        Database {
            uri: Url::parse("postgres://example:example@example:1234/example_app").unwrap(),
            #[cfg(feature = "test-containers")]
            test_container: None,
            auto_migrate: true,
            connect_timeout: Duration::from_secs(1),
            connect_lazy: true,
            acquire_timeout: Duration::from_secs(2),
            idle_timeout: Some(Duration::from_secs(3)),
            max_lifetime: Some(Duration::from_secs(4)),
            min_connections: 10,
            max_connections: 20,
            test_on_checkout: true,
            #[cfg(feature = "db-diesel-pool-async")]
            retry_connection: true,
            temporary_test_db: false,
            temporary_test_db_clean_up: false,
        }
    }

    #[rstest::rstest]
    #[cfg(feature = "db-sea-orm")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn db_config_to_connect_options(db_config: Database) {
        let connect_options = sea_orm::ConnectOptions::from(db_config);

        insta::assert_debug_snapshot!(connect_options);
    }

    #[rstest::rstest]
    #[cfg(feature = "db-sea-orm")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn db_config_to_connect_options_ref(db_config: Database) {
        let connect_options = sea_orm::ConnectOptions::from(&db_config);

        insta::assert_debug_snapshot!(connect_options);
    }
}
