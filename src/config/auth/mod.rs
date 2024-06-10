use crate::util::serde_util::UriOrString;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Auth {
    #[validate(nested)]
    pub jwt: Jwt,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Jwt {
    pub secret: String,
    #[serde(default)]
    #[validate(nested)]
    pub claims: JwtClaims,
}

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct JwtClaims {
    // Todo: Default to the server URL?
    #[serde(default)]
    pub audience: Vec<UriOrString>,
    /// Claim names to require, in addition to the default-required `exp` claim.
    #[serde(default)]
    pub required_claims: Vec<String>,
}

#[cfg(test)]
mod tests {
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
        [jwt]
        secret = "foo"
        "#
    )]
    #[case(
        r#"
        [jwt]
        secret = "foo"
        [jwt.claims]
        audience = ["bar"]
        "#
    )]
    #[case(
        r#"
        [jwt]
        secret = "foo"
        [jwt.claims]
        required-claims = ["baz"]
        "#
    )]
    #[case(
        r#"
        [jwt]
        secret = "foo"
        [jwt.claims]
        audience = ["bar"]
        required-claims = ["baz"]
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn auth(_case: TestCase, #[case] config: &str) {
        let auth: Auth = toml::from_str(config).unwrap();

        assert_toml_snapshot!(auth);
    }
}
