use std::fmt::Display;
use std::str::FromStr;

use serde::{de, Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};
use url::Url;

/// Custom deserializer to allow deserializing a string field as the given type type [T], as long as
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

/// Custom deserializer to allow serializing the given type [T] as a string, as long as the type
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
