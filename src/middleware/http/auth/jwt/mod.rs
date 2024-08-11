#[cfg(feature = "jwt-ietf")]
pub mod ietf;
#[cfg(feature = "jwt-openid")]
pub mod openid;

use crate::app::context::AppContext;
use crate::error::api::http::HttpError;
use crate::error::{Error, RoadsterResult};
#[cfg(feature = "jwt-ietf")]
use crate::middleware::http::auth::jwt::ietf::Claims;
#[cfg(all(feature = "jwt-openid", not(feature = "jwt-ietf")))]
use crate::middleware::http::auth::jwt::openid::Claims;
use crate::util::serde::{deserialize_from_str, serialize_to_str};
use async_trait::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::RequestPartsExt;
use axum_extra::extract::CookieJar;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::{Authorization, HeaderValue};
use axum_extra::TypedHeader;
use itertools::Itertools;
use jsonwebtoken::{decode, DecodingKey, Header, TokenData, Validation};
use serde_derive::{Deserialize, Serialize};
#[cfg(not(any(feature = "jwt-ietf", feature = "jwt-openid")))]
use serde_json::Value as Claims;
use url::Url;
use uuid::Uuid;

type BearerAuthHeader = TypedHeader<Authorization<Bearer>>;

/// Struct representing a JWT, including its [Header]s and `claims`. The `claims` type (`C`) can be
/// customized. If features `jwt-ietf` or `jwt-openid` are enabled, the type will default to
/// the claims for the respective feature. If both features are enabled, the type will default
/// to the claims from `jwt-ietf`. If neither feature is enabled (but `jwt` is enabled), then
/// the default will simply be a [serde_json::Value]. In all cases, the type can be overridden
/// by the consumer.
#[cfg_attr(feature = "open-api", derive(aide::OperationIo))]
#[derive(Deserialize, Serialize)]
#[non_exhaustive]
pub struct Jwt<C = Claims> {
    pub header: Header,
    pub claims: C,
}

#[async_trait]
impl<S, C> FromRequestParts<S> for Jwt<C>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    C: for<'de> serde::Deserialize<'de>,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let context = AppContext::from_ref(state);

        let token = parts
            .extract::<BearerAuthHeader>()
            .await
            .ok()
            .map(|auth_header| auth_header.0.token().to_string());
        let token = if token.is_some() {
            token
        } else {
            let cookies = parts.extract::<CookieJar>().await.ok();
            bearer_token_from_cookies(&context, cookies)
        };

        let token = if let Some(token) = token {
            token
        } else {
            return Err(HttpError::unauthorized().into());
        };

        let token: TokenData<C> = decode_auth_token(
            &token,
            &context.config().auth.jwt.secret,
            &context.config().auth.jwt.claims.audience,
            &context.config().auth.jwt.claims.required_claims,
        )?;
        let token = Jwt {
            header: token.header,
            claims: token.claims,
        };
        Ok(token)
    }
}

fn bearer_token_from_cookies(context: &AppContext, cookies: Option<CookieJar>) -> Option<String> {
    let cookie_name = &context.config().auth.jwt.cookie_name;
    cookies
        .as_ref()
        .and_then(|cookies| cookies.get(cookie_name))
        .map(|cookie| cookie.value())
        .and_then(|token| HeaderValue::from_str(token).ok())
        .and_then(|header_value| {
            <Authorization<Bearer> as axum_extra::headers::Header>::decode(
                &mut [&header_value].into_iter(),
            )
            .ok()
        })
        .map(|auth_header| auth_header.token().to_string())
}

fn decode_auth_token<T1, T2, C>(
    token: &str,
    jwt_secret: &str,
    audience: &[T1],
    required_claims: &[T2],
) -> RoadsterResult<TokenData<C>>
where
    T1: ToString,
    T2: ToString,
    C: for<'de> serde::Deserialize<'de>,
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
    let token_data: TokenData<C> = decode(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )?;
    Ok(token_data)
}

/// The subject of a JWT claim. Technically the IETF spec only specifies that this is a `StringOrURI`
/// type, and the OpenID spec specifies String. However, since this is likely to contain a user ID,
/// we will also try to deserialize directly into a UUID or Integer. Deserialization will fall back
/// to a simple String if the value can not be parsed into a UUID or Integer (or URI).
/// See: <https://www.rfc-editor.org/rfc/rfc7519.html#section-4.1.2>
/// See: <https://openid.net/specs/openid-connect-core-1_0.html#IDToken>
// Intentionally not annotated with `#[non_exhaustive]`
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
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

impl From<Uuid> for Subject {
    fn from(value: Uuid) -> Self {
        Subject::Uuid(value)
    }
}

impl From<u8> for Subject {
    fn from(value: u8) -> Self {
        Subject::Int(value as u64)
    }
}

impl From<u16> for Subject {
    fn from(value: u16) -> Self {
        Subject::Int(value as u64)
    }
}

impl From<u32> for Subject {
    fn from(value: u32) -> Self {
        Subject::Int(value as u64)
    }
}

impl From<u64> for Subject {
    fn from(value: u64) -> Self {
        Subject::Int(value)
    }
}

impl From<Url> for Subject {
    fn from(value: Url) -> Self {
        Subject::Uri(value)
    }
}

impl From<String> for Subject {
    fn from(value: String) -> Self {
        if let Ok(value) = value.parse::<Url>() {
            value.into()
        } else if let Ok(value) = value.parse::<Uuid>() {
            value.into()
        } else if let Ok(value) = value.parse::<u64>() {
            value.into()
        } else {
            Subject::String(value)
        }
    }
}

impl From<&str> for Subject {
    fn from(value: &str) -> Self {
        if let Ok(value) = value.parse::<Url>() {
            value.into()
        } else if let Ok(value) = value.parse::<Uuid>() {
            value.into()
        } else if let Ok(value) = value.parse::<u64>() {
            value.into()
        } else {
            Subject::String(value.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use crate::util::serde::Wrapper;
    use axum::http::header::AUTHORIZATION;
    use axum_extra::extract::cookie::Cookie;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use serde_json::from_str;
    use std::str::FromStr;
    use url::Url;

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case::valid_token("Bearer foo")]
    #[case::invalid_token("foo")]
    fn bearer_token_from_cookies(_case: TestCase, #[case] cookie_value: &str) {
        let context = AppContext::test(None, None, None).unwrap();

        let cookies = CookieJar::new().add(Cookie::new(
            AUTHORIZATION.to_string(),
            cookie_value.to_string(),
        ));

        let token = super::bearer_token_from_cookies(&context, Some(cookies));

        assert_debug_snapshot!(token);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_subject_as_uri() {
        let value: Wrapper<Subject> = from_str(r#"{"inner": "https://example.com"}"#).unwrap();
        assert_eq!(
            value.inner,
            Subject::Uri(Url::from_str("https://example.com").unwrap())
        );
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_uri() {
        let subject: Subject = Url::from_str("https://example.com").unwrap().into();
        assert_debug_snapshot!(subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_subject_as_uuid() {
        let uuid = Uuid::new_v4();
        let value: Wrapper<Subject> = from_str(&format!(r#"{{"inner": "{uuid}"}}"#)).unwrap();
        assert_eq!(value.inner, Subject::Uuid(uuid));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_uuid() {
        let _case = case();

        let subject: Subject = Uuid::new_v4().into();
        assert_debug_snapshot!(subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_subject_as_int() {
        let num = 100;
        let value: Wrapper<Subject> = from_str(&format!(r#"{{"inner": "{num}"}}"#)).unwrap();
        assert_eq!(value.inner, Subject::Int(num));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_u8() {
        let _case = case();

        let subject: Subject = 12u8.into();
        assert_debug_snapshot!(subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_u16() {
        let _case = case();

        let subject: Subject = 1234u16.into();
        assert_debug_snapshot!(subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_u32() {
        let _case = case();

        let subject: Subject = 1234u32.into();
        assert_debug_snapshot!(subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_u64() {
        let _case = case();

        let subject: Subject = 1234u64.into();
        assert_debug_snapshot!(subject);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialize_subject_int_as_string() {
        let num = 100;
        let value = Wrapper {
            inner: Subject::Int(num),
        };
        let s = serde_json::to_string(&value).unwrap();
        assert_eq!(s, format!(r#"{{"inner":"{num}"}}"#));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_subject_as_string() {
        let value: Wrapper<Subject> = from_str(r#"{"inner": "invalid-uri"}"#).unwrap();
        assert_eq!(value.inner, Subject::String("invalid-uri".to_string()));
    }

    #[rstest]
    #[case("http://example.com".to_string())]
    #[case(Uuid::new_v4().to_string())]
    #[case("1234".to_string())]
    #[case("foo".to_string())]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn subject_from_string(_case: TestCase, #[case] value: String) {
        let subject_from_str: Subject = value.as_str().into();
        let subject: Subject = value.into();

        assert_eq!(subject, subject_from_str);
        assert_debug_snapshot!(subject);
    }
}
