use crate::util::serde::default_true;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Sendgrid {
    pub api_key: String,
    #[serde(default = "default_true")]
    pub https_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
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
        api-key = "api-key"
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        http_only = false
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        http_only = true
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let sendgrid: Sendgrid = toml::from_str(config).unwrap();

        assert_toml_snapshot!(sendgrid);
    }
}
