//! Utilities for modifying `insta` snapshot [Settings].

use crate::util::regex::UUID_REGEX;
use insta::internals::SettingsBindDropGuard;
use insta::Settings;
use std::thread::current;
use typed_builder::TypedBuilder;

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

/// Redact instances of UUIDs in snapshots. Applies a filter on the [Settings] to replace
/// sub-strings matching [UUID_REGEX] with `[uuid]`.
pub fn snapshot_redact_uuid(settings: &mut Settings) -> &mut Settings {
    settings.add_filter(UUID_REGEX, "[uuid]");
    settings
}

/// Extract the last segment of the current thread name to use as the test case description.
///
/// See: <https://github.com/adriangb/pgpq/blob/b0b0f8c77c862c0483d81571e76f3a2b746136fc/pgpq/src/lib.rs#L649-L669>
/// See: <https://github.com/la10736/rstest/issues/177>
fn description_from_current_thread() -> String {
    let thread_name = current().name().unwrap_or("").to_string();
    let description = thread_name
        .split("::")
        .map(|item| item.split('_').skip(2).collect::<Vec<&str>>().join("_"))
        .last()
        .filter(|s| !s.is_empty())
        .unwrap_or(thread_name.split("::").last().unwrap().to_string());
    description
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use rstest::rstest;
    use uuid::Uuid;

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
    fn uuid() {
        let _case = TestCase::new();

        let uuid = Uuid::new_v4();

        assert_snapshot!(format!("Foo '{uuid}' bar"));
    }
}
