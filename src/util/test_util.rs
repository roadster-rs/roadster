use insta::internals::SettingsBindDropGuard;
use std::thread::current;

/// See: https://insta.rs/docs/patterns/
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn set_snapshot_suffix(suffix: &str) -> SettingsBindDropGuard {
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_suffix(suffix);
    settings.bind_to_scope()
}

pub struct TestCase {
    pub description: String,
    _settings_guard: SettingsBindDropGuard,
}

impl TestCase {
    pub fn new() -> Self {
        test_case()
    }
}

impl Default for TestCase {
    fn default() -> Self {
        TestCase::new()
    }
}

/// See: https://github.com/adriangb/pgpq/blob/b0b0f8c77c862c0483d81571e76f3a2b746136fc/pgpq/src/lib.rs#L649-L669
/// See: https://github.com/la10736/rstest/issues/177
#[cfg_attr(coverage_nightly, coverage(off))]
fn test_case() -> TestCase {
    let name = current().name().unwrap().to_string();
    let description = name
        .split("::")
        .map(|item| item.split('_').skip(2).collect::<Vec<&str>>().join("_"))
        .last()
        .filter(|s| !s.is_empty())
        .unwrap_or(name.split("::").last().unwrap().to_string());
    TestCase {
        _settings_guard: set_snapshot_suffix(&description),
        description,
    }
}
