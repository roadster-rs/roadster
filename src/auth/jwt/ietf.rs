use crate::auth::jwt::Subject;
use crate::util::serde_util::UriOrString;
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
#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.1>
    #[serde(rename = "iss")]
    pub issuer: Option<UriOrString>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.2>
    #[serde(rename = "sub")]
    pub subject: Option<Subject>,

    /// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.3>
    #[serde(rename = "aud", default, skip_serializing_if = "Vec::is_empty")]
    #[serde_as(deserialize_as = "serde_with::OneOrMany<_>")]
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
    pub jwt_id: Option<String>,

    #[serde(flatten)]
    pub custom: BTreeMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use std::ops::{Add, Sub};
    use std::str::FromStr;

    use chrono::{TimeDelta, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header, TokenData};
    use serde_derive::{Deserialize, Serialize};
    use serde_json::from_str;
    use url::Url;

    use crate::auth::jwt::decode_auth_token;
    use crate::auth::jwt::ietf::{Claims, Subject};
    use crate::util::serde_util::UriOrString;

    const TEST_JWT_SECRET: &str = "test-jwt-secret";
    const AUDIENCE: &[&str] = &["authenticated"];
    const REQUIRED_CLAIMS: &[&str] = &[];

    #[test]
    fn test_decode_token() {
        let jwt = build_token(false, None);

        let decoded: TokenData<Claims> =
            decode_auth_token(&jwt.1, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS).unwrap();

        assert_eq!(decoded.claims.subject, jwt.0.subject);
    }

    #[test]
    fn test_decode_token_expired() {
        let (_, jwt) = build_token(true, None);

        let decoded: anyhow::Result<TokenData<Claims>> =
            decode_auth_token(&jwt, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS);

        assert!(decoded.is_err());
    }

    #[test]
    fn test_decode_token_wrong_audience() {
        let (_, jwt) = build_token(false, Some("different-audience".to_string()));

        let decoded: anyhow::Result<TokenData<Claims>> =
            decode_auth_token(&jwt, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS);

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

    #[derive(Debug, Deserialize, Serialize)]
    struct Wrapper<T> {
        inner: T,
    }

    #[test]
    fn deserialize_audience_as_vec() {
        let value: Wrapper<Vec<UriOrString>> =
            from_str(r#"{"inner": ["https://example.com", "aud2"]}"#).unwrap();
        assert_eq!(
            value.inner,
            vec![
                UriOrString::Uri(Url::from_str("https://example.com").unwrap()),
                UriOrString::String("aud2".to_string())
            ]
        );
    }
}
