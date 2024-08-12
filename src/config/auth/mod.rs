use crate::util::serde::UriOrString;
use axum::http::header::AUTHORIZATION;
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
    /// Name of the cookie used to pass the JWT access token. If not set, will use
    /// [`AUTHORIZATION`] as the cookie name.
    #[serde(default = "Jwt::default_cookie_name")]
    #[deprecated(
        since = "0.5.19",
        note = "Using jwt from cookie is/may be a CSRF vulnerability. This functionality is removed for now and this config field is not used."
    )]
    pub cookie_name: String,

    pub secret: String,

    #[serde(default)]
    #[validate(nested)]
    pub claims: JwtClaims,
}

impl Jwt {
    fn default_cookie_name() -> String {
        AUTHORIZATION.as_str().to_string()
    }
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
