use crate::error::Error;
use std::net::AddrParseError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ParseError {
    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Address(#[from] AddrParseError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<url::ParseError> for Error {
    fn from(value: url::ParseError) -> Self {
        Self::Parse(ParseError::from(value))
    }
}

impl From<AddrParseError> for Error {
    fn from(value: AddrParseError) -> Self {
        Self::Parse(ParseError::from(value))
    }
}
