use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::RequestPartsExt;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use jsonwebtoken::{decode, DecodingKey, Header, TokenData, Validation};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use url::Url;
use uuid::Uuid;

use crate::app_context::AppContext;
use crate::util::serde_util::{deserialize_from_str, serialize_to_str, UriOrString};
use crate::view::app_error::AppError;

type BearerAuthHeader = TypedHeader<Authorization<Bearer>>;

pub struct Jwt {
    pub header: Header,
    // Todo: Other Claims types?
    // Todo: Make Claims type generic?
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for Jwt
where
    S: Into<Arc<AppContext>> + Clone + Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts.extract::<BearerAuthHeader>().await?;
        let state: Arc<AppContext> = state.clone().into();
        let token = decode_auth_token(
            auth_header.0.token(),
            &state.config.auth.jwt.secret,
            &state.config.auth.jwt.claims.audience,
            &state.config.auth.jwt.claims.required_claims,
        )?;
        let token = Jwt {
            header: token.header,
            claims: token.claims,
        };
        Ok(token)
    }
}

fn decode_auth_token<T1, T2>(
    token: &str,
    jwt_secret: &str,
    audience: &[T1],
    required_claims: &[T2],
) -> anyhow::Result<TokenData<Claims>>
where
    T1: ToString,
    T2: ToString,
{
    let mut validation = Validation::default();
    validation.set_audience(audience);
    if !required_claims.is_empty() {
        // Todo: Is there a way to reduce the allocations used here?
        let required_claims = validation
            .required_spec_claims
            .iter()
            .map(|claim| claim.to_string())
            .chain(required_claims.iter().map(|claim| claim.to_string()))
            .collect_vec();
        validation.set_required_spec_claims(&required_claims);
    }
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )?;
    Ok(token_data)
}

/// JWT Claims. Provides fields for the default/recommended registered claim names. Additional
/// claim names are collected in the `custom` map.
/// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4>
// Todo: Add fields for OpenID registered claim names: https://www.iana.org/assignments/jwt/jwt.xhtml#claims
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
    /// Not technically required by the JWT spec, but is required by the default [Validation] we use.
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

/// The subject of a JWT claim. Technically the spec only specifies that this is a `StringOrURI`
/// type. However, since this is likely to contain a user ID, we will also try to deserialize
/// directly into a UUID or Integer. Deserialization will fall back to a simple String if
/// the value can not be parsed into a UUID or Integer (or URI).
/// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.2>
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Subject {
    Uri(Url),
    Uuid(Uuid),
    Int(
        #[serde(
            deserialize_with = "deserialize_from_str",
            serialize_with = "serialize_to_str"
        )]
        u64,
    ),
    String(String),
}

#[cfg(test)]
mod tests {
    use std::ops::{Add, Sub};
    use std::str::FromStr;

    use chrono::{TimeDelta, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_derive::{Deserialize, Serialize};
    use serde_json::from_str;
    use url::Url;

    use crate::auth::jwt::{decode_auth_token, Claims, Subject};
    use crate::util::serde_util::UriOrString;

    const TEST_JWT_SECRET: &str = "test-jwt-secret";
    const AUDIENCE: &[&str] = &["authenticated"];
    const REQUIRED_CLAIMS: &[&str] = &[];

    #[test]
    fn test_decode_token() {
        let jwt = build_token(false, None);

        let decoded =
            decode_auth_token(&jwt.1, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS).unwrap();

        assert_eq!(decoded.claims.subject, jwt.0.subject);
    }

    #[test]
    fn test_decode_token_expired() {
        let (_, jwt) = build_token(true, None);

        let decoded = decode_auth_token(&jwt, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS);

        assert!(decoded.is_err());
    }

    #[test]
    fn test_decode_token_wrong_audience() {
        let (_, jwt) = build_token(false, Some("different-audience".to_string()));

        let decoded = decode_auth_token(&jwt, TEST_JWT_SECRET, AUDIENCE, REQUIRED_CLAIMS);

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
    fn deserialize_uri_or_string_as_uri() {
        let value: Wrapper<UriOrString> = from_str(r#"{"inner": "https://example.com"}"#).unwrap();
        assert_eq!(
            value.inner,
            UriOrString::Uri(Url::from_str("https://example.com").unwrap())
        );
    }

    #[test]
    fn serialize_uri_as_string() {
        let value = Wrapper {
            inner: UriOrString::Uri(Url::from_str("https://example.com").unwrap()),
        };
        let s = serde_json::to_string(&value).unwrap();
        assert_eq!(s, r#"{"inner":"https://example.com/"}"#);
    }

    #[test]
    fn deserialize_uri_or_string_as_string() {
        let value: Wrapper<UriOrString> = from_str(r#"{"inner": "invalid-uri"}"#).unwrap();
        assert_eq!(value.inner, UriOrString::String("invalid-uri".to_string()));
    }

    #[test]
    fn deserialize_subject_as_uri() {
        let value: Wrapper<Subject> = from_str(r#"{"inner": "https://example.com"}"#).unwrap();
        assert_eq!(
            value.inner,
            Subject::Uri(Url::from_str("https://example.com").unwrap())
        );
    }

    #[test]
    fn deserialize_subject_as_uuid() {
        let uuid = uuid::Uuid::new_v4();
        let value: Wrapper<Subject> = from_str(&format!(r#"{{"inner": "{uuid}"}}"#)).unwrap();
        assert_eq!(value.inner, Subject::Uuid(uuid));
    }

    #[test]
    fn deserialize_subject_as_int() {
        let num = 100;
        let value: Wrapper<Subject> = from_str(&format!(r#"{{"inner": "{num}"}}"#)).unwrap();
        assert_eq!(value.inner, Subject::Int(num));
    }

    #[test]
    fn serialize_subject_int_as_string() {
        let num = 100;
        let value = Wrapper {
            inner: Subject::Int(num),
        };
        let s = serde_json::to_string(&value).unwrap();
        assert_eq!(s, format!(r#"{{"inner":"{num}"}}"#));
    }

    #[test]
    fn deserialize_subject_as_string() {
        let value: Wrapper<Subject> = from_str(r#"{"inner": "invalid-uri"}"#).unwrap();
        assert_eq!(value.inner, Subject::String("invalid-uri".to_string()));
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
