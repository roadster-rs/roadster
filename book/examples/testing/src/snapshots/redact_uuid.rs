use insta::assert_snapshot;
use roadster::testing::snapshot::TestCase;
use uuid::Uuid;

#[test]
fn normalize_dynamic_uuid() {
    let _case = TestCase::default();

    let uuid = Uuid::new_v4();
    assert_snapshot!(uuid);
}
