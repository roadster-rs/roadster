use crate::util::serde::default_true;
use sea_orm::ConnectOptions;
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
    /// Whether to attempt to connect to the DB immediately upon creating the [`ConnectOptions`].
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
}

impl Database {
    fn default_connect_timeout() -> Duration {
        Duration::from_millis(1000)
    }

    fn default_acquire_timeout() -> Duration {
        Duration::from_millis(1000)
    }
}

impl From<Database> for ConnectOptions {
    fn from(database: Database) -> Self {
        ConnectOptions::from(&database)
    }
}

impl From<&Database> for ConnectOptions {
    fn from(database: &Database) -> Self {
        let mut options = ConnectOptions::new(database.uri.to_string());
        options
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

#[cfg(test)]
mod deserialize_tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::{assert_debug_snapshot, assert_toml_snapshot};
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sidekiq(_case: TestCase, #[case] config: &str) {
        let database: Database = toml::from_str(config).unwrap();

        assert_toml_snapshot!(database);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn db_config_to_connect_options() {
        let db = Database {
            uri: Url::parse("postgres://example:example@example:1234/example_app").unwrap(),
            auto_migrate: true,
            connect_timeout: Duration::from_secs(1),
            connect_lazy: true,
            acquire_timeout: Duration::from_secs(2),
            idle_timeout: Some(Duration::from_secs(3)),
            max_lifetime: Some(Duration::from_secs(4)),
            min_connections: 10,
            max_connections: 20,
        };

        let connect_options = ConnectOptions::from(&db);

        assert_debug_snapshot!(connect_options);
    }
}
