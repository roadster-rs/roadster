use crate::middleware::http::auth::jwt::Subject;
use crate::util::serde::UriOrString;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use std::collections::BTreeMap;

/// JWT Claims. Provides fields for the default/recommended registered claim names. Additional
/// claim names are collected in the `custom` map.
/// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4>
#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize, bon::Builder)]
#[non_exhaustive]
pub struct Claims<C = BTreeMap<String, Value>> {
    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.1>
    #[serde(rename = "iss")]
    pub issuer: Option<UriOrString>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.2>
    #[serde(rename = "sub")]
    pub subject: Option<Subject>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.3>
    #[serde(rename = "aud", default, skip_serializing_if = "Vec::is_empty")]
    #[serde_as(deserialize_as = "serde_with::OneOrMany<_>")]
    #[builder(default)]
    pub audience: Vec<UriOrString>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.4>
    /// Not technically required by the JWT spec, but is required by the default [jsonwebtoken::Validation] we use.
    #[serde(rename = "exp", with = "ts_seconds")]
    pub expires_at: DateTime<Utc>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.5>
    #[serde(rename = "nbf")]
    #[serde_as(as = "Option<serde_with::TimestampSeconds>")]
    pub not_before: Option<DateTime<Utc>>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.6>
    #[serde(rename = "iat")]
    #[serde_as(as = "Option<serde_with::TimestampSeconds>")]
    pub issued_at: Option<DateTime<Utc>>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.7>
    #[serde(rename = "jti")]
    #[builder(into)]
    pub jwt_id: Option<String>,

    #[serde(flatten)]
    pub custom: C,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::RoadsterResult;
    use crate::middleware::http::auth::jwt::decode_auth_token_internal;
    use crate::util::serde::{UriOrString, Wrapper};
    use chrono::{TimeDelta, Utc};
    use insta::assert_debug_snapshot;
    use jsonwebtoken::{EncodingKey, Header, TokenData, encode};
    use std::ops::{Add, Sub};
    use std::str::FromStr;
    use url::Url;

    const TEST_JWT_SECRET: &str = "test-jwt-secret";
    const AUDIENCE: &[&str] = &["authenticated"];
    const REQUIRED_CLAIMS: &[&str] = &[];

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn decode_token() {
        let jwt = build_token(false, None);

        let decoded: TokenData<Claims> =
            decode_auth_token_internal(&jwt.1, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS).unwrap();

        assert_eq!(decoded.claims.subject, jwt.0.subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn decode_token_expired() {
        let (_, jwt) = build_token(true, None);

        let decoded: RoadsterResult<TokenData<Claims>> =
            decode_auth_token_internal(&jwt, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS);

        assert!(decoded.is_err());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn decode_token_wrong_audience() {
        let (_, jwt) = build_token(false, Some("different-audience".to_string()));

        let decoded: RoadsterResult<TokenData<Claims>> =
            decode_auth_token_internal(&jwt, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS);

        assert!(decoded.is_err());
    }

    fn build_token(expired: bool, audience: Option<String>) -> (Claims, String) {
        let (expires_at, issued_at) = if expired {
            (
                Utc::now().sub(TimeDelta::try_minutes(30).unwrap()),
                Utc::now().sub(TimeDelta::try_minutes(2).unwrap()),
            )
        } else {
            (Utc::now().add(TimeDelta::try_hours(1).unwrap()), Utc::now())
        };
        let claims = Claims {
            issuer: Some(UriOrString::Uri(
                Url::from_str("https://example.com").unwrap(),
            )),
            subject: Some(Subject::Uuid(uuid::Uuid::new_v4())),
            audience: vec![UriOrString::String(
                audience.unwrap_or_else(|| "authenticated".to_string()),
            )],
            expires_at,
            issued_at: Some(issued_at),
            not_before: None,
            jwt_id: None,
            custom: Default::default(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(TEST_JWT_SECRET.as_ref()),
        )
        .unwrap();
        (claims, token)
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_audience_as_vec() {
        let value: Wrapper<Vec<UriOrString>> =
            serde_json::from_str(r#"{"inner": ["https://example.com", "aud2"]}"#).unwrap();
        assert_eq!(
            value.inner,
            vec![
                UriOrString::Uri(Url::from_str("https://example.com").unwrap()),
                UriOrString::String("aud2".to_string())
            ]
        );
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_claims() {
        let claims = r#"
        exp = 1000
        "#;
        let claims: Claims = toml::from_str(claims).unwrap();
        assert_debug_snapshot!(claims);
    }
}
