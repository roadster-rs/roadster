use crate::middleware::http::auth::jwt::Subject;
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;
use std::collections::BTreeMap;
use url::Url;

use crate::util::serde_util::{deserialize_from_str, serialize_to_str, UriOrString};

/// JWT Claims. Provides fields for the default/recommended registered claim names. Additional
/// claim names are collected in the `custom` map.
/// See: <https://openid.net/specs/openid-connect-core-1_0.html#IDToken>
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    #[serde(rename = "iss")]
    pub issuer: Url,

    #[serde(rename = "sub")]
    pub subject: Subject,

    #[serde(rename = "aud", default, skip_serializing_if = "Vec::is_empty")]
    #[serde_as(deserialize_as = "serde_with::OneOrMany<_>")]
    pub audience: Vec<UriOrString>,

    #[serde(rename = "exp", with = "ts_seconds")]
    pub expires_at: DateTime<Utc>,

    #[serde(rename = "iat", with = "ts_seconds")]
    pub issued_at: DateTime<Utc>,

    #[serde_as(as = "Option<serde_with::TimestampSeconds>")]
    pub auth_time: Option<DateTime<Utc>>,

    pub nonce: Option<String>,

    #[serde(rename = "acr")]
    pub auth_cxt_class_reference: Option<Acr>,

    #[serde(rename = "amr", default, skip_serializing_if = "Vec::is_empty")]
    pub auth_methods_references: Vec<String>,

    /// Note per the OpenID docs: "\[...\] in practice, the azp Claim only occurs when extensions
    /// beyond the scope of this specification are used; therefore, implementations not using such
    /// extensions are encouraged to not use azp and to ignore it when it does occur."
    #[serde(rename = "azp")]
    pub authorized_party: Option<UriOrString>,

    #[serde(flatten)]
    pub custom: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Acr {
    Uri(Url),
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
    use crate::util::serde_util::Wrapper;
    use serde_json::from_str;
    use std::str::FromStr;
    use url::Url;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_acr_as_uri() {
        let value: Wrapper<Acr> = from_str(r#"{"inner": "https://example.com"}"#).unwrap();
        assert_eq!(
            value.inner,
            Acr::Uri(Url::from_str("https://example.com").unwrap())
        );
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_acr_as_int() {
        let num = 100;
        let value: Wrapper<Acr> = from_str(&format!(r#"{{"inner": "{num}"}}"#)).unwrap();
        assert_eq!(value.inner, Acr::Int(num));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialize_arc_int_as_string() {
        let num = 100;
        let value = Wrapper {
            inner: Acr::Int(num),
        };
        let s = serde_json::to_string(&value).unwrap();
        assert_eq!(s, format!(r#"{{"inner":"{num}"}}"#));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_acr_as_string() {
        let value: Wrapper<Acr> = from_str(r#"{"inner": "invalid-uri"}"#).unwrap();
        assert_eq!(value.inner, Acr::String("invalid-uri".to_string()));
    }
}
