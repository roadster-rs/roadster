use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Duration;
use url::Url;
use validator::Validate;

#[serde_as]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Database {
    /// This can be overridden with an environment variable, e.g. `ROADSTER.DATABASE.URI=postgres://example:example@example:1234/example_app`
    pub uri: Url,
    /// Whether to automatically apply migrations during the app's start up. Migrations can also
    /// be manually performed via the `roadster migration [COMMAND]` CLI command.
    pub auto_migrate: bool,
    #[serde(default = "Database::default_connect_timeout")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub connect_timeout: Duration,
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

#[cfg(test)]
mod deserialize_tests {
    use super::*;
    use crate::util::test_util::TestCase;
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
}
