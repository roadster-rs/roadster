use crate::error::Error;
#[cfg(feature = "http")]
use axum::http::StatusCode;
#[cfg(feature = "http")]
use axum::response::{IntoResponse, Response};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AuthError {
    #[cfg(feature = "jwt")]
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[cfg(feature = "http")]
    #[error(transparent)]
    Bearer(#[from] axum_extra::headers::authorization::InvalidBearerToken),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "jwt")]
impl From<jsonwebtoken::errors::Error> for Error {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        Self::Auth(AuthError::from(value))
    }
}

#[cfg(feature = "http")]
impl From<axum_extra::headers::authorization::InvalidBearerToken> for Error {
    fn from(value: axum_extra::headers::authorization::InvalidBearerToken) -> Self {
        Self::Auth(AuthError::from(value))
    }
}

#[cfg(feature = "http")]
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
