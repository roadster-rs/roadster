use crate::error::Error;

#[derive(Debug, Error)]
pub enum AxumError {
    #[error(transparent)]
    InvalidHeader(#[from] axum::http::header::InvalidHeaderName),

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

#[cfg(feature = "jwt")]
impl From<axum_extra::typed_header::TypedHeaderRejection> for Error {
    fn from(value: axum_extra::typed_header::TypedHeaderRejection) -> Self {
        Self::Axum(AxumError::from(value))
    }
}
