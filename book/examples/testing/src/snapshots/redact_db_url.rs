use insta::assert_snapshot;
use roadster::testing::snapshot::TestCase;

#[test]
fn redact_sensitive_db_url() {
    let _case = TestCase::default();

    let db_url = "postgres://example.com:1234";
    assert_snapshot!(db_url);
}
