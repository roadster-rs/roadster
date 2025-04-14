use insta::assert_snapshot;
use roadster::testing::snapshot::TestCase;
use rstest::{fixture, rstest};

// File path: <crate_name>/src/snapshots/rstest.rs

#[fixture]
fn case() -> TestCase {
    Default::default()
}

#[rstest]
#[case(1)]
#[case(2)]
#[case::case_name(3)]
fn normalize_dynamic_uuid(_case: TestCase, #[case] value: u32) {
    assert_snapshot!(value);
}
