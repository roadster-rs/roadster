use crate::util::serde::default_true;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
#[cfg(any(feature = "db-sea-orm", feature = "worker-pg"))]
use std::str::FromStr;
use std::time::Duration;
use url::Url;
use validator::Validate;

#[serde_as]
#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Database {
    /// This can be overridden with an environment variable, e.g. `ROADSTER__DATABASE__URI=postgres://example:example@example:1234/example_app`
    pub uri: Url,

    /// Whether to automatically apply migrations during the app's start up. Migrations can also
    /// be manually performed via the `roadster migration [COMMAND]` CLI command.
    pub auto_migrate: bool,

    /// Create a temporary database in the same DB host from the `uri` field.
    #[serde(default)]
    pub temporary_test_db: bool,

    /// Automatically clean up (drop) the temporary test DB that was created by setting
    /// `temporary_test_db` to `true`. Note that the test DB will only be cleaned up if the closure
    /// passed to [`crate::app::run_test`] or [`crate::app::run_test_with_result`] doesn't panic.
    #[cfg(feature = "testing")]
    #[serde(default = "default_true")]
    pub temporary_test_db_clean_up: bool,

    #[validate(nested)]
    #[serde(flatten)]
    pub pool_config: DbPoolConfig,

    #[validate(nested)]
    #[serde(default, flatten)]
    #[cfg(any(feature = "worker-pg", feature = "db-sea-orm"))]
    pub statement_log_config: StatementLogConfig,

    /// Options for creating a Test Container instance for the DB. If enabled, the `Database#uri`
    /// field will be overridden to be the URI for the Test Container instance that's created when
    /// building the app's [`crate::app::context::AppContext`].
    #[cfg(feature = "test-containers")]
    #[serde(default)]
    #[validate(nested)]
    pub test_container: Option<crate::config::TestContainer>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct DbPoolConfig {
    #[serde(default = "DbPoolConfig::default_connect_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub connect_timeout: Duration,

    /// Whether to attempt to connect to the DB immediately when the DB connection pool is created.
    /// If `true` will wait to connect to the DB until the first DB query is attempted.
    #[serde(default = "default_true")]
    pub connect_lazy: bool,

    #[serde(default = "DbPoolConfig::default_acquire_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub acquire_timeout: Duration,

    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub idle_timeout: Option<Duration>,

    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub max_lifetime: Option<Duration>,

    #[serde(default)]
    pub min_connections: u32,

    pub max_connections: u32,

    #[serde(default = "default_true")]
    pub test_on_checkout: bool,

    /// See [`bb8::Builder::retry_connection`]
    #[cfg(feature = "db-diesel-pool-async")]
    #[serde(default = "default_true")]
    pub retry_connection: bool,
}

impl DbPoolConfig {
    pub(crate) fn default_connect_timeout() -> Duration {
        Duration::from_millis(1000)
    }

    pub(crate) fn default_acquire_timeout() -> Duration {
        Duration::from_millis(1000)
    }
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
#[cfg(any(feature = "db-sea-orm", feature = "worker-pg"))]
pub struct StatementLogConfig {
    #[serde(default)]
    pub enable_statement_logging: bool,
    #[serde(default)]
    #[validate(custom(function = "valid_level_filter"))]
    pub statement_log_level: Option<String>,
    #[serde(default)]
    #[validate(custom(function = "valid_level_filter"))]
    pub slow_statement_log_level: Option<String>,
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub slow_statement_duration_threshold: Option<std::time::Duration>,
}

#[cfg(any(feature = "db-sea-orm", feature = "worker-pg"))]
fn valid_level_filter(level: &str) -> Result<(), validator::ValidationError> {
    log::LevelFilter::from_str(level).map_err(|err| {
        let mut validation_error = validator::ValidationError::new("Invalid level filter");
        validation_error.add_param("level".into(), &level);
        validation_error.add_param("error".into(), &err.to_string());
        validation_error
    })?;
    Ok(())
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
            .test_before_acquire(database.pool_config.test_on_checkout)
            .connect_timeout(database.pool_config.connect_timeout)
            .connect_lazy(database.pool_config.connect_lazy)
            .acquire_timeout(database.pool_config.acquire_timeout)
            .min_connections(database.pool_config.min_connections)
            .max_connections(database.pool_config.max_connections);

        if let Some(level) = database.statement_log_config.statement_log_level.as_ref()
            && let Ok(level) = log::LevelFilter::from_str(level)
        {
            options.sqlx_logging_level(level);
        }

        if let Some((level, duration)) = database
            .statement_log_config
            .slow_statement_log_level
            .as_ref()
            .zip(
                database
                    .statement_log_config
                    .slow_statement_duration_threshold
                    .as_ref(),
            )
            && let Ok(level) = log::LevelFilter::from_str(level)
        {
            options.sqlx_slow_statements_logging_settings(level, *duration);
        }

        options.sqlx_logging(database.statement_log_config.enable_statement_logging);

        if let Some(idle_timeout) = database.pool_config.idle_timeout {
            options.idle_timeout(idle_timeout);
        }
        if let Some(max_lifetime) = database.pool_config.max_lifetime {
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
            .test_before_acquire(value.pool_config.test_on_checkout)
            .acquire_timeout(value.pool_config.acquire_timeout)
            .min_connections(value.pool_config.min_connections)
            .max_connections(value.pool_config.max_connections)
            .idle_timeout(value.pool_config.idle_timeout)
            .max_lifetime(value.pool_config.max_lifetime)
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
    #[cfg(all(
        feature = "db-diesel-pool-async",
        any(feature = "worker-pg", feature = "db-sea-orm")
    ))]
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
            pool_config: DbPoolConfig {
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
            },
            #[cfg(any(feature = "worker-pg", feature = "db-sea-orm"))]
            statement_log_config: Default::default(),
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
