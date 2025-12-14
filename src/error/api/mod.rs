#[cfg(feature = "http")]
pub mod http;

use crate::error::Error;
#[cfg(feature = "http")]
use crate::error::api::http::HttpError;
#[cfg(feature = "http")]
use axum::http::StatusCode;
#[cfg(feature = "http")]
use axum::response::{IntoResponse, Response};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ApiError {
    #[cfg(feature = "http")]
    #[error(transparent)]
    Http(HttpError<serde_json::Value>),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

#[cfg(feature = "http")]
impl<T> From<HttpError<T>> for Error
where
    T: serde::Serialize,
{
    fn from(value: HttpError<T>) -> Self {
        Self::Api(ApiError::Http(value.details_serialized()))
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
