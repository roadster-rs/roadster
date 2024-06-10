use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AxumError {
    #[error(transparent)]
    InvalidHeaderName(#[from] axum::http::header::InvalidHeaderName),

    #[error(transparent)]
    InvalidHeaderValue(#[from] axum::http::header::InvalidHeaderValue),

    #[error(transparent)]
    InvalidMethod(#[from] axum::http::method::InvalidMethod),

    #[cfg(feature = "jwt")]
    #[error(transparent)]
    TypedHeaderRejection(#[from] axum_extra::typed_header::TypedHeaderRejection),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<axum::http::header::InvalidHeaderName> for Error {
    fn from(value: axum::http::header::InvalidHeaderName) -> Self {
        Self::Axum(AxumError::from(value))
    }
}

impl From<axum::http::header::InvalidHeaderValue> for Error {
    fn from(value: axum::http::header::InvalidHeaderValue) -> Self {
        Self::Axum(AxumError::from(value))
    }
}

impl From<axum::http::method::InvalidMethod> for Error {
    fn from(value: axum::http::method::InvalidMethod) -> Self {
        Self::Axum(AxumError::from(value))
    }
}

#[cfg(feature = "jwt")]
impl From<axum_extra::typed_header::TypedHeaderRejection> for Error {
    fn from(value: axum_extra::typed_header::TypedHeaderRejection) -> Self {
        Self::Axum(AxumError::from(value))
    }
}
