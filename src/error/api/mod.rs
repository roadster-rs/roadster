#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "http")]
use crate::error::api::http::HttpError;
use crate::error::Error;
#[cfg(feature = "http")]
use axum::http::StatusCode;
#[cfg(feature = "http")]
use axum::response::{IntoResponse, Response};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiError {
    #[cfg(feature = "http")]
    #[error(transparent)]
    Http(HttpError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "http")]
impl From<HttpError> for Error {
    fn from(value: HttpError) -> Self {
        Self::Api(ApiError::Http(value))
    }
}

#[cfg(feature = "http")]
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Http(err) => err.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
