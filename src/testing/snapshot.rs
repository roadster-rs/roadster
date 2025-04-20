//! Utilities for modifying `insta` snapshot [Settings].

use crate::util::regex::UUID_REGEX;
use insta::Settings;
use insta::internals::SettingsBindDropGuard;
use itertools::Itertools;
use regex::Regex;
use std::str::FromStr;
use std::sync::LazyLock;
use std::thread::current;
use typed_builder::TypedBuilder;

const BEARER_TOKEN_REGEX: &str = r"Bearer [\w\.-]+";
const POSTGRES_URI_REGEX: &str = r"postgres://(\w|\d|@|:|\/|\.)+";
const MYSQL_URI_REGEX: &str = r"mysql://(\w|\d|@|:|\/|\.)+";
const REDIS_URI_REGEX: &str = r"redis://(\w|\d|@|:|\/|\.)+";
const SMTP_URI_REGEX: &str = r"smtp://(\w|\d|@|:|\/|\.)+";
// https://stackoverflow.com/questions/3143070/regex-to-match-an-iso-8601-datetime-string
const TIMESTAMP_REGEX: &str = r"(\d{4}-[01]\d-[0-3]\d\s?T?\s?[0-2]\d:[0-5]\d:[0-5]\d\.\d+\s?([+-][0-2]\d:[0-5]\d|Z))|(\d{4}-[01]\d-[0-3]\d\s?T?\s?[0-2]\d:[0-5]\d:[0-5]\d\s?([+-][0-2]\d:[0-5]\d|Z))|(\d{4}-[01]\d-[0-3]\d\s?T?\s?[0-2]\d:[0-5]\d\s?([+-][0-2]\d:[0-5]\d|Z))";

/// Configure which settings to apply on the snapshot [Settings].
///
/// When built, a [TestCase] is returned.
#[derive(TypedBuilder)]
#[builder(build_method(into = TestCase))]
#[non_exhaustive]
pub struct TestCaseConfig {
    /// The [Settings] to modify. If not provided, will use `Settings::clone_current()`.
    #[builder(default, setter(strip_option))]
    pub settings: Option<Settings>,

    /// The description of the test case. If not provided, will be extracted from the name of
    /// the current thread, which is based on the test case name.
    ///
    /// This is particularly useful when using `insta` together with `rstest`.
    /// See: <https://insta.rs/docs/patterns/>
    ///
    /// # Examples
    ///
    /// ## [TestCase] description for `rstest` cases
    /// ```rust
    /// #[cfg(test)]
    /// mod tests {
    ///     use insta::assert_snapshot;
    ///     use rstest::{fixture, rstest};
    ///     use roadster::testing::snapshot::TestCase;
    ///
    ///     #[fixture]
    ///     fn case() -> TestCase {
    ///         Default::default()
    ///     }
    ///
    ///     #[rstest]
    ///     #[case(0)] // _case.description == case_1
    ///     #[case::foo(0)] // _case.description == foo
    ///     fn test(_case: TestCase, #[case] num: u32) {
    ///         // Snapshot file name will have suffix of `@{_case.description}`, e.g. `@case_1`
    ///         assert_snapshot!(num);
    ///     }
    /// }
    /// ```
    ///
    /// ## [TestCase] with manually set description
    /// ```rust
    /// #[cfg(test)]
    /// mod tests {
    ///     use insta::assert_snapshot;
    ///     use roadster::testing::snapshot::{TestCase, TestCaseConfig};
    ///
    ///     #[test]
    ///     fn test() {
    ///         let _case = TestCaseConfig::builder().description("Custom description").build();
    ///         // Snapshot file name will have suffix of `@Custom description`
    ///         assert_snapshot!("snapshot_value");
    ///     }
    /// }
    /// ```
    #[builder(default, setter(strip_option, into))]
    pub description: Option<String>,

    /// Whether to set the `description` as the suffix of the snapshot file.
    ///
    /// It is particularly useful to set this to `true` when using `insta` together with `rstest`.
    /// See: <https://insta.rs/docs/patterns/>
    #[builder(default = true)]
    pub set_suffix: bool,

    /// Whether to redact UUIDs from snapshots. This is useful for tests involving
    /// dynamically created UUIDs that will be different on every test run, or involve real UUIDs
    /// that you don't want leaked in your source code.
    #[builder(default = true)]
    pub redact_uuid: bool,

    /// Whether to redact auth tokens from snapshots. This is useful for tests involving
    /// dynamically created auth tokens that will be different on every test run, or involve real
    /// auth tokens that you don't want leaked in your source code.
    #[builder(default = true)]
    pub redact_auth_tokens: bool,

    /// Whether to redact Postgres URIs from snapshots. This is useful for tests involving
    /// dynamically created Postgres instances that will be different on every test run, or involve
    /// real Postgres instances that you don't want leaked in your source code.
    #[builder(default = true)]
    pub redact_postgres_uri: bool,

    /// Whether to redact Mysql URIs from snapshots. This is useful for tests involving
    /// dynamically created Mysql instances that will be different on every test run, or involve
    /// real Mysql instances that you don't want leaked in your source code.
    #[builder(default = true)]
    pub redact_mysql_uri: bool,

    /// Whether to redact Redis URIs from snapshots. This is useful for tests involving
    /// dynamically created Redis instances that will be different on every test run, or involve
    /// real Redis instances that you don't want leaked in your source code.
    #[builder(default = true)]
    pub redact_redis_uri: bool,

    /// Whether to redact SMTP URIs from snapshots. This is useful for tests involving
    /// dynamically created SMTP instances that will be different on every test run, or involve
    /// real SMTP instances that you don't want leaked in your source code.
    #[builder(default = true)]
    pub redact_smtp_uri: bool,

    /// Whether to redact timestamps. This is useful for tests involving
    /// dynamically created timestamps that will be different on every test run.
    #[builder(default = true)]
    pub redact_timestamp: bool,

    /// Whether to automatically bind the [Settings] to the current scope. If `true`, the settings
    /// will be automatically applied for the test in which the [TestCase] was built. If `false`,
    /// the settings will only be applied after manually calling [Settings::bind_to_scope], or
    /// placing all relevant snapshot assertions inside a [Settings::bind] call.
    ///
    /// # Examples
    ///
    /// ## Auto bind to scope
    /// ```rust
    /// #[cfg(test)]
    /// mod tests {
    ///     use insta::assert_snapshot;
    ///     use roadster::testing::snapshot::{TestCase, TestCaseConfig};
    ///
    ///     #[test]
    ///     fn test() {
    ///         let _case = TestCaseConfig::builder().description("Custom description").build();
    ///         // Snapshot file name will have suffix of `@Custom description`
    ///         assert_snapshot!("snapshot_value");
    ///     }
    /// }
    /// ```
    ///
    /// ## Manually bind [Settings] scope
    /// ```rust
    /// #[cfg(test)]
    /// mod tests {
    ///     use insta::assert_snapshot;
    ///     use roadster::testing::snapshot::{TestCase, TestCaseConfig};
    ///
    ///     #[test]
    ///     fn test() {
    ///         let case = TestCaseConfig::builder().bind_scope(false).build();
    ///         // This snapshot will not have a suffix
    ///         assert_snapshot!("snapshot_value");
    ///
    ///         case.settings.bind(|| {
    ///             // This snapshot will have suffix `@test` (extracted from the curren thread name)
    ///             assert_snapshot!("snapshot_value_2");
    ///         });
    ///     }
    /// }
    /// ```
    #[builder(default = true)]
    pub bind_scope: bool,
}

/// Container for common `insta` snapshot [Settings] after they have been applied per the
/// [TestCaseConfig].
#[non_exhaustive]
pub struct TestCase {
    /// The description of the current test case. Either manually provided via the
    /// [TestCaseConfigBuilder::description], or extracted from the current thread name.
    pub description: String,
    /// The `insta` [Settings] that are configured.
    pub settings: Settings,
    _settings_guard: Option<SettingsBindDropGuard>,
}

impl TestCase {
    pub fn new() -> Self {
        TestCaseConfig::builder().build()
    }
}

impl Default for TestCase {
    fn default() -> Self {
        TestCase::new()
    }
}

impl From<TestCaseConfig> for TestCase {
    fn from(value: TestCaseConfig) -> Self {
        let mut settings = value.settings.unwrap_or(Settings::clone_current());

        let description = value
            .description
            .unwrap_or(description_from_current_thread());

        if value.set_suffix {
            snapshot_set_suffix(&mut settings, &description);
        }
        if value.redact_uuid {
            snapshot_redact_uuid(&mut settings);
        }
        if value.redact_auth_tokens {
            snapshot_redact_bearer_tokens(&mut settings);
        }
        if value.redact_postgres_uri {
            snapshot_redact_postgres_uri(&mut settings);
        }
        if value.redact_mysql_uri {
            snapshot_redact_mysql_uri(&mut settings);
        }
        if value.redact_redis_uri {
            snapshot_redact_redis_uri(&mut settings);
        }
        if value.redact_smtp_uri {
            snapshot_redact_smtp_uri(&mut settings);
        }
        if value.redact_timestamp {
            snapshot_redact_timestamp(&mut settings);
        }

        let _settings_guard = if value.bind_scope {
            Some(settings.bind_to_scope())
        } else {
            None
        };

        Self {
            description,
            settings,
            _settings_guard,
        }
    }
}

/// Set the snapshot suffix on the [Settings].
///
/// Useful for using `insta` together with `rstest`.
/// See: <https://insta.rs/docs/patterns/>
pub fn snapshot_set_suffix<'a>(settings: &'a mut Settings, suffix: &str) -> &'a mut Settings {
    settings.set_snapshot_suffix(suffix);
    settings
}

/// Redact instances of UUIDs in snapshots. Applies a filter on the [`Settings`] to replace
/// sub-strings matching [`UUID_REGEX`] with `[uuid]`.
pub fn snapshot_redact_uuid(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(UUID_REGEX, "[uuid]");
    settings
}

/// Redact instances of bearer tokens in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [`BEARER_TOKEN_REGEX`] with `Sensitive`.
pub fn snapshot_redact_bearer_tokens(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(BEARER_TOKEN_REGEX, "Sensitive");
    settings
}

/// Redact instances of Postgres URIs in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [`POSTGRES_URI_REGEX`] with `postgres://[Sensitive]`.
pub fn snapshot_redact_postgres_uri(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(POSTGRES_URI_REGEX, "postgres://[Sensitive]");
    settings
}

/// Redact instances of Mysql URIs in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [`MYSQL_URI_REGEX`] with `mysql://[Sensitive]`.
pub fn snapshot_redact_mysql_uri(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(MYSQL_URI_REGEX, "mysql://[Sensitive]");
    settings
}

/// Redact instances of Redis URIs in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [`REDIS_URI_REGEX`] with `redis://[Sensitive]`.
pub fn snapshot_redact_redis_uri(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(REDIS_URI_REGEX, "redis://[Sensitive]");
    settings
}

/// Redact instances of Smtp URIs in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [`SMTP_URI_REGEX`] with `smtp://[Sensitive]`.
pub fn snapshot_redact_smtp_uri(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(SMTP_URI_REGEX, "smtp://[Sensitive]");
    settings
}

/// Redact instances of timestamps in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [`TIMESTAMP_REGEX`] with `[timestamp]`.
pub fn snapshot_redact_timestamp(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(TIMESTAMP_REGEX, "[timestamp]");
    settings
}

/// Extract the last segment of the current thread name to use as the test case description.
///
/// See: <https://github.com/adriangb/pgpq/blob/b0b0f8c77c862c0483d81571e76f3a2b746136fc/pgpq/src/lib.rs#L649-L669>
/// See: <https://github.com/la10736/rstest/issues/177>
pub(crate) fn description_from_current_thread() -> String {
    let thread_name = current().name().unwrap_or("").to_string();
    description_from_thread_name(&thread_name)
}

fn description_from_thread_name(name: &str) -> String {
    let description = name
        .split("::")
        .map(|item| {
            if item.starts_with("case_") {
                item.split('_').skip(2).join("_")
            } else {
                item.to_string()
            }
        })
        .last()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback_description(name));

    description
}

const CASE_PREFIX: &str = "case_";
static CASE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::expect_used)]
    Regex::from_str(&format!(r"{CASE_PREFIX}(\d+)")).expect("Unable to parse regex")
});

fn fallback_description(name: &str) -> String {
    #[allow(clippy::expect_used)]
    let last = name
        .split("::")
        .last()
        .expect("No string segments after splitting by `::`")
        .to_string();
    CASE_REGEX
        .captures(&last)
        .and_then(|captures| captures.get(1))
        .map(|m| format!("{CASE_PREFIX}{:0>2}", m.as_str()))
        .unwrap_or(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(0, false)]
    #[case::rstest_description(1, false)]
    #[case(2, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn description(#[case] num: u32, #[case] manual_description: bool) {
        let _case = if manual_description {
            TestCaseConfig::builder()
                .description("manual_description")
                .build()
        } else {
            TestCase::new()
        };

        assert_snapshot!(num);
    }

    #[rstest]
    #[case(0, false, false)]
    #[case(1, true, false)]
    #[case(2, false, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn bind_scope(
        #[case] num: u32,
        #[case] auto_bind_scope: bool,
        #[case] manual_bind_scope: bool,
    ) {
        assert!(!(auto_bind_scope && manual_bind_scope));

        let case = TestCaseConfig::builder()
            .bind_scope(auto_bind_scope)
            .build();

        if manual_bind_scope {
            case.settings.bind(|| {
                assert_snapshot!(num);
            })
        } else {
            assert_snapshot!(num);
        }
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn uuid() {
        let _case = TestCase::new();

        let uuid = Uuid::new_v4();

        assert_snapshot!(format!("Foo '{uuid}' bar"));
    }

    #[rstest]
    #[case("Bearer 1234")]
    #[case("Bearer access-token")]
    #[case("Bearer some.jwt.token")]
    #[case("Bearer foo-bar.baz-1234")]
    #[case("Bearer token;")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn bearer_token(_case: TestCase, #[case] token: &str) {
        assert_snapshot!(format!("Foo {token} bar"));
    }

    #[rstest]
    #[case("postgres://example:example@example.com:1234/example")]
    #[case("postgres://example:1234")]
    #[case("postgres://localhost")]
    #[case("postgres://example.com")]
    #[case("postgres://192.168.1.1:3000")]
    #[case("postgres://192.168.1.1:3000/example")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn postgres_uri(_case: TestCase, #[case] uri: &str) {
        assert_snapshot!(format!("uri = {uri}"));
    }

    #[rstest]
    #[case("mysql://example:example@example.com:1234/example")]
    #[case("mysql://example:1234")]
    #[case("mysql://localhost")]
    #[case("mysql://example.com")]
    #[case("mysql://192.168.1.1:3000")]
    #[case("mysql://192.168.1.1:3000/example")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn mysql_uri(_case: TestCase, #[case] uri: &str) {
        assert_snapshot!(format!("uri = {uri}"));
    }

    #[rstest]
    #[case("redis://example:example@example.com:1234/example")]
    #[case("redis://example:1234")]
    #[case("redis://localhost")]
    #[case("redis://example.com")]
    #[case("redis://192.168.1.1:3000")]
    #[case("redis://192.168.1.1:3000/example")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn redis_uri(_case: TestCase, #[case] uri: &str) {
        assert_snapshot!(format!("uri = {uri}"));
    }

    #[rstest]
    #[case("smtp://example:example@example.com:1234/example")]
    #[case("smtp://example:1234")]
    #[case("smtp://localhost")]
    #[case("smtp://example.com")]
    #[case("smtp://192.168.1.1:3000")]
    #[case("smtp://192.168.1.1:3000/example")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn smtp_uri(_case: TestCase, #[case] uri: &str) {
        assert_snapshot!(format!("uri = {uri}"));
    }

    #[rstest]
    #[case("")]
    #[case("foo")]
    #[case("foo::bar")]
    #[case("foo::bar::x_y_z_1_2_3")]
    #[case("foo::bar::case_1_x_y_z_1_2_3")]
    #[case("foo::bar::case_1")]
    #[case("foo::bar::case_11")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn description_from_thread_name(_case: TestCase, #[case] name: &str) {
        let description = super::description_from_thread_name(name);

        assert_snapshot!(description);
    }

    #[rstest]
    #[case("case_1")]
    #[case("foo::bar::case_1")]
    #[case("foo::bar::case_11")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fallback_description(_case: TestCase, #[case] name: &str) {
        let description = super::description_from_thread_name(name);

        assert_snapshot!(description);
    }
}
