use crate::util::serde::UriOrString;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Auth {
    #[validate(nested)]
    #[cfg(feature = "jwt")]
    pub jwt: Jwt,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[cfg(feature = "jwt")]
pub struct Jwt {
    /// Name of the cookie used to pass the JWT access token. If provided, the default
    /// [`Jwt`][crate::middleware::http::auth::jwt::Jwt] will extract the access token from the
    /// provided cookie name if it wasn't present in the `authorization`
    /// request header. If not provided, the extractor will only consider the request header.
    ///
    /// Warning: Providing this field opens up an application to CSRF vulnerabilities unless the
    /// application has the proper protections in place. See the following for more information:
    /// - <https://owasp.org/www-community/attacks/csrf>
    /// - <https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html>
    pub cookie_name: Option<String>,

    pub secret: String,

    #[serde(default)]
    #[validate(nested)]
    pub claims: JwtClaims,
}

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
#[cfg(feature = "jwt")]
pub struct JwtClaims {
    // Todo: Default to the server URL?
    #[serde(default)]
    pub audience: Vec<UriOrString>,
    /// Claim names to require, in addition to the default-required `exp` claim.
    #[serde(default)]
    pub required_claims: Vec<String>,
}

#[cfg(all(test, feature = "jwt"))]
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
    #[case(
        r#"
        [jwt]
        secret = "foo"
        cookie-name = "authorization"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn auth(_case: TestCase, #[case] config: &str) {
        let auth: Auth = toml::from_str(config).unwrap();

        assert_toml_snapshot!(auth);
    }
}
