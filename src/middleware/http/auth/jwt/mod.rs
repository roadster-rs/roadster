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
use axum::RequestPartsExt;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum_extra::TypedHeader;
use axum_extra::extract::CookieJar;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Bearer;
use itertools::Itertools;
use jsonwebtoken::{DecodingKey, Header, TokenData, Validation, decode};
use serde_derive::{Deserialize, Serialize};
#[cfg(not(any(feature = "jwt-ietf", feature = "jwt-openid")))]
use serde_json::Value as Claims;
use url::Url;
use uuid::Uuid;

type BearerAuthHeader = TypedHeader<Authorization<Bearer>>;

/// Struct representing a JWT, including its [`Header`]s and `claims`. The `claims` type (`C`) can be
/// customized. If features `jwt-ietf` or `jwt-openid` are enabled, the type will default to
/// the claims for the respective feature. If both features are enabled, the type will default
/// to the claims from `jwt-ietf`. If neither feature is enabled (but `jwt` is enabled), then
/// the default will simply be a [`serde_json::Value`]. In all cases, the type can be overridden
/// by the consumer.
#[cfg_attr(feature = "open-api", derive(aide::OperationIo))]
#[derive(Deserialize, Serialize)]
#[non_exhaustive]
pub struct Jwt<C = Claims> {
    pub header: Header,
    pub claims: C,
}

impl<S, C> FromRequestParts<S> for Jwt<C>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    C: for<'de> serde::Deserialize<'de> + Clone,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_from_request_parts_maybe_cookie(parts, state, false).await?;
        Ok(token.token)
    }
}

/// Similar to [`Jwt`], but allows extracting the JWT from the request cookies (if the
/// `auth.jwt.cookie-name` config is set) in addition to the request header.
///
/// This is useful for use in web-apps because the JWT can be set as a cookie and be automatically
/// sent along with every request instead of needing to add the header to every request (which would
/// preclude the web-app from supporting clients without javascript enabled). However, *THIS MAY
/// MAKE THE CONSUMING APPLICATION VULNERABLE TO CSRF ATTACKS*. If this struct is used, the
/// consuming application should implement a CSRF protection mechanism. See the following for more
/// information and recommendations:
///
/// - <https://owasp.org/www-community/attacks/csrf>
/// - <https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html>
///
/// If the functionality to extract from a cookie is not required, it's recommended to use
/// the normal [`Jwt`] directly.
#[cfg_attr(feature = "open-api", derive(aide::OperationIo))]
#[serde_with::skip_serializing_none]
#[derive(Deserialize, Serialize, bon::Builder)]
#[non_exhaustive]
pub struct JwtCsrf<C = Claims> {
    pub token: Jwt<C>,
    pub csrf_status: CsrfStatus,
}

impl<S, C> FromRequestParts<S> for JwtCsrf<C>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    C: for<'de> serde::Deserialize<'de> + Clone,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = extract_from_request_parts_maybe_cookie(parts, state, true).await?;
        Ok(token)
    }
}

/// Included in [`JwtCsrf`] to indicate whether it's safe to use the JWT or if the consuming
/// application should apply a CSRF protection mechanism before performing any destructive actions
/// for the subject represented by the JWT.
#[cfg_attr(feature = "open-api", derive(aide::OperationIo))]
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[non_exhaustive]
pub enum CsrfStatus {
    /// Indicates that the consuming application should apply a CSRF protection mechanism before
    /// performing any destructive actions for the subject represented by the [`JwtCsrf`].
    Vulnerable,
    /// Indicates that the consuming application can safely use the [`JwtCsrf`] without applying
    /// a CSRF protection mechanism first.
    Safe,
}

async fn extract_from_request_parts_maybe_cookie<S, C>(
    parts: &mut Parts,
    state: &S,
    allow_extract_from_cookie: bool,
) -> RoadsterResult<JwtCsrf<C>>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    C: for<'de> serde::Deserialize<'de> + Clone,
{
    let context = AppContext::from_ref(state);

    let token = parts
        .extract::<BearerAuthHeader>()
        .await
        .ok()
        .map(|auth_header| auth_header.0.token().to_string());

    let (token, csrf_status) = if token.is_some() {
        (token, Some(CsrfStatus::Safe))
    } else if !allow_extract_from_cookie {
        (None, None)
    } else if let Some(cookie_name) = context.config().auth.jwt.cookie_name.as_ref() {
        let token = parts
            .extract::<CookieJar>()
            .await
            .ok()
            .and_then(|cookies| token_from_cookies(cookie_name, cookies));
        (token, Some(CsrfStatus::Vulnerable))
    } else {
        (None, None)
    };

    let (token, csrf_status) = if let Some((token, csrf_status)) = token.zip(csrf_status) {
        (token, csrf_status)
    } else {
        return Err(HttpError::unauthorized()
            .error("Authorization token not found.")
            .into());
    };

    let token = decode_auth_token(state, &token)?;

    Ok(JwtCsrf::builder()
        .token(token)
        .csrf_status(csrf_status)
        .build())
}

fn token_from_cookies(cookie_name: &str, cookies: CookieJar) -> Option<String> {
    cookies
        .get(cookie_name)
        .map(|cookie| cookie.value().to_string())
}

pub fn decode_auth_token<S, C>(state: &S, token: &str) -> RoadsterResult<Jwt<C>>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    C: for<'de> serde::Deserialize<'de> + Clone,
{
    let context = AppContext::from_ref(state);
    let token: TokenData<C> = decode_auth_token_internal(
        token,
        &context.config().auth.jwt.secret,
        &context.config().auth.jwt.claims.audience,
        &context.config().auth.jwt.claims.required_claims,
    )?;

    Ok(Jwt {
        header: token.header,
        claims: token.claims,
    })
}

fn decode_auth_token_internal<T1, T2, C>(
    token: &str,
    jwt_secret: &str,
    audience: &[T1],
    required_claims: &[T2],
) -> RoadsterResult<TokenData<C>>
where
    T1: ToString,
    T2: ToString,
    C: for<'de> serde::Deserialize<'de> + Clone,
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
    use crate::config::AppConfig;
    use crate::testing::snapshot::TestCase;
    use crate::util::serde::Wrapper;
    use axum::http::Request;
    use axum::http::header::{AUTHORIZATION, COOKIE};
    use axum_core::body::Body;
    use axum_extra::extract::cookie::Cookie;
    use chrono::{Duration, Utc};
    use insta::{assert_debug_snapshot, assert_json_snapshot};
    use rstest::{fixture, rstest};
    use serde_json::from_str;
    use std::collections::BTreeMap;
    use std::str::FromStr;
    use url::Url;

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case::valid_token("foo")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn token_from_cookies(_case: TestCase, #[case] cookie_value: &str) {
        let cookies = CookieJar::new().add(Cookie::new(
            AUTHORIZATION.as_str(),
            cookie_value.to_string(),
        ));

        let token = super::token_from_cookies(AUTHORIZATION.as_str(), cookies);

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

    #[fixture]
    #[once]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn context() -> AppContext {
        let mut config = AppConfig::test(None).unwrap();
        config.auth.jwt.claims.required_claims = vec!["sub".to_string()];
        AppContext::test(Some(config), None, None).unwrap()
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn token(context: &AppContext) -> String {
        let subject = Uuid::new_v4().to_string();

        let claims = Claims::<BTreeMap<String, String>>::builder()
            .subject(subject.into())
            .expires_at(Utc::now() + Duration::days(1))
            .custom(Default::default())
            .build();
        jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(context.config().auth.jwt.secret.as_ref()),
        )
        .unwrap()
    }

    #[rstest]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn jwt_from_request_parts(_case: TestCase, token: String, context: &AppContext) {
        let request: Request<Body> = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(().into())
            .unwrap();

        let jwt = Jwt::<Claims>::from_request_parts(&mut request.into_parts().0, context)
            .await
            .unwrap();

        assert_json_snapshot!(jwt, { ".claims.exp" => 1234 });
    }

    #[rstest]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn jwt_from_request_parts_cookie(_case: TestCase, token: String, context: &AppContext) {
        let mut config = context.config().clone();
        config.auth.jwt.cookie_name = Some("authorization".to_string());
        let context = AppContext::test(Some(config), None, None).unwrap();
        let request: Request<Body> = Request::builder()
            .header(COOKIE, format!("authorization={token}"))
            .body(().into())
            .unwrap();

        let jwt = Jwt::<Claims>::from_request_parts(&mut request.into_parts().0, &context).await;

        assert!(jwt.is_err());
    }

    #[rstest]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn jwt_csrf_from_request_parts(_case: TestCase, token: String, context: &AppContext) {
        let request: Request<Body> = Request::builder()
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(().into())
            .unwrap();

        let jwt = JwtCsrf::<Claims>::from_request_parts(&mut request.into_parts().0, context)
            .await
            .unwrap();

        assert_json_snapshot!(jwt, { ".token.claims.exp" => 1234 });
    }

    #[rstest]
    #[case(None)]
    #[case(Some("authorization".to_string()))]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn jwt_csrf_from_request_parts_cookie(
        _case: TestCase,
        token: String,
        context: &AppContext,
        #[case] cookie_name: Option<String>,
    ) {
        let mut config = context.config().clone();
        config.auth.jwt.cookie_name = cookie_name.clone();
        let context = AppContext::test(Some(config), None, None).unwrap();
        let request: Request<Body> = Request::builder()
            .header(COOKIE, format!("authorization={token}"))
            .body(().into())
            .unwrap();

        let jwt =
            JwtCsrf::<Claims>::from_request_parts(&mut request.into_parts().0, &context).await;

        assert_eq!(jwt.is_err(), cookie_name.is_none());
    }
}
