#[cfg(feature = "jwt-ietf")]
pub mod ietf;
#[cfg(feature = "jwt-openid")]
pub mod openid;

#[mockall_double::double]
use crate::app_context::AppContext;
#[cfg(feature = "jwt-ietf")]
use crate::auth::jwt::ietf::Claims;
#[cfg(all(feature = "jwt-openid", not(feature = "jwt-ietf")))]
use crate::auth::jwt::openid::Claims;
use crate::util::serde_util::{deserialize_from_str, serialize_to_str};
use crate::view::app_error::AppError;
#[cfg(feature = "open-api")]
use aide::OperationInput;
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::RequestPartsExt;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
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
pub struct Jwt<C = Claims>
where
    C: for<'de> serde::Deserialize<'de>,
{
    pub header: Header,
    pub claims: C,
}

// Required in order to use `Jwt` in an Aide route.
#[cfg(feature = "open-api")]
impl OperationInput for Jwt {}

#[async_trait]
impl<S, C> FromRequestParts<AppContext<S>> for Jwt<C>
where
    S: Send + Sync,
    C: for<'de> serde::Deserialize<'de>,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppContext<S>,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts.extract::<BearerAuthHeader>().await?;
        let token: TokenData<C> = decode_auth_token(
            auth_header.0.token(),
            &state.config().auth.jwt.secret,
            &state.config().auth.jwt.claims.audience,
            &state.config().auth.jwt.claims.required_claims,
        )?;
        let token = Jwt {
            header: token.header,
            claims: token.claims,
        };
        Ok(token)
    }
}

fn decode_auth_token<T1, T2, C>(
    token: &str,
    jwt_secret: &str,
    audience: &[T1],
    required_claims: &[T2],
) -> anyhow::Result<TokenData<C>>
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
    use super::*;
    use serde_derive::{Deserialize, Serialize};
    use serde_json::from_str;
    use std::str::FromStr;
    use url::Url;

    #[derive(Debug, Deserialize, Serialize)]
    struct Wrapper<T> {
        inner: T,
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
}
