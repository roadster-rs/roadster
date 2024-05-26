use crate::error::Error;
#[cfg(feature = "http")]
use axum::http::StatusCode;
#[cfg(feature = "http")]
use axum::response::{IntoResponse, Response};

#[derive(Debug, Error)]
pub enum AuthError {
    #[cfg(feature = "jwt")]
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),

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
impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
