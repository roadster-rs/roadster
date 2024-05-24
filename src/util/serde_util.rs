use std::fmt::Display;
use std::str::FromStr;

use serde::de::IntoDeserializer;
use serde::{de, Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};
use serde_json::{Map, Value};
use url::Url;

/// Custom deserializer to allow deserializing a string field as the given type `T`, as long as
/// the type implements [FromStr].
pub fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    match T::from_str(&s) {
        Ok(v) => Ok(v),
        Err(_) => Err(de::Error::custom(
            "String could not be parsed as the desired type.",
        )),
    }
}

/// Custom serializer to allow serializing the given type `T` as a string, as long as the type
/// implements [FromStr].
pub fn serialize_to_str<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: ToString,
{
    serializer.serialize_str(&value.to_string())
}

/// Type that can be used to deserialize a value to a URI or, if the value is not a valid URI, fall
/// back to deserializing as a string.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum UriOrString {
    Uri(Url),
    String(String),
}

impl Display for UriOrString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UriOrString::Uri(inner) => write!(f, "{}", inner),
            UriOrString::String(inner) => write!(f, "{}", inner),
        }
    }
}

/// Function to default a boolean field to `true`.
pub const fn default_true() -> bool {
    true
}

// This method isn't used for some feature combinations
#[allow(dead_code)]
pub(crate) fn empty_json_object() -> impl for<'de> Deserializer<'de> {
    Value::Object(Map::new()).into_deserializer()
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_uri_or_string_as_uri() {
        let value: Wrapper<UriOrString> = from_str(r#"{"inner": "https://example.com"}"#).unwrap();
        assert_eq!(
            value.inner,
            UriOrString::Uri(Url::from_str("https://example.com").unwrap())
        );
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialize_uri_as_string() {
        let value = Wrapper {
            inner: UriOrString::Uri(Url::from_str("https://example.com").unwrap()),
        };
        let s = serde_json::to_string(&value).unwrap();
        assert_eq!(s, r#"{"inner":"https://example.com/"}"#);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn uri_or_string_uri_variant_to_string() {
        let uri = UriOrString::Uri(Url::from_str("https://example.com").unwrap());
        assert_eq!("https://example.com/", uri.to_string());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn uri_or_string_string_variant_to_string() {
        let uri = UriOrString::String("foo".to_string());
        assert_eq!("foo", uri.to_string());
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_uri_or_string_as_string() {
        let value: Wrapper<UriOrString> = from_str(r#"{"inner": "invalid-uri"}"#).unwrap();
        assert_eq!(value.inner, UriOrString::String("invalid-uri".to_string()));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_true_returns_true() {
        assert!(default_true());
    }
}
