use crate::error::Error;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Error type representing an HTTP API error. This is generally expected to be returned explicitly
/// by your application logic.
///
/// # Examples
///
/// ## Alternative -- directly use `StatusCode`
/// If you simply need to create an Axum response with just a status code, this class is not
/// necessary. You can instead use `StatusCode` directly:
///
/// ```rust
/// use axum::http::StatusCode;
/// use axum::response::IntoResponse;
///
/// fn api_method() -> impl IntoResponse {
///     StatusCode::BAD_REQUEST
/// }
/// ```
///
/// This can also work when your api method returns a result, either with a generic response:
///
/// ```rust
/// use axum::http::StatusCode;
/// use axum::response::IntoResponse;
///
/// fn api_method() -> Result<(), impl IntoResponse> {
///     Err(StatusCode::BAD_REQUEST)
/// }
/// ```
///
/// Or when returning a [roadster result][crate::error::RoadsterResult] (which uses a
/// [roadster error][enum@Error] for its `Error` type).
///
/// ```rust
/// use axum::http::StatusCode;
/// use axum::response::IntoResponse;
/// use roadster::error::RoadsterResult;
///
/// fn api_method() -> RoadsterResult<()> {
///     Err(StatusCode::BAD_REQUEST.into())
/// }
/// ```
///
/// ## Create from `StatusCode`
///
/// ```rust
/// use axum::http::StatusCode;
/// use roadster::error::api::http::HttpError;
///
/// let err: HttpError = StatusCode::BAD_REQUEST.into();
/// ```
///
/// ## Create using a helper method
///
/// ```rust
/// use roadster::error::api::http::HttpError;
///
/// let err = HttpError::bad_request();
/// ```
///
/// ## Populate additional fields with builder-style methods
///
/// ```rust
/// use roadster::error::api::http::HttpError;
///
/// let err = HttpError::bad_request()
///     .error("Something went wrong")
///     .details("Field 'A' is missing");
/// ```
///
/// ## Using in an API method
///
/// ```rust
/// use axum::response::IntoResponse;
/// use roadster::error::api::http::HttpError;
///
/// fn api_method() -> Result<(), impl IntoResponse> {
///     let err = HttpError::bad_request()
///         .error("Something went wrong")
///         .details("Field 'A' is missing");
///     Err(err)
/// }
/// ```
///
/// ## Using in an API method that returns `RoadsterResult`
///
/// ```rust
/// use axum::response::IntoResponse;
/// use roadster::error::api::http::HttpError;
/// use roadster::error::RoadsterResult;
///
/// fn api_method() -> RoadsterResult<()> {
///     let err = HttpError::bad_request()
///         .error("Something went wrong")
///         .details("Field 'A' is missing");
///     Err(err.into())
/// }
/// ```
///
#[derive(Debug, Error, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(aide::OperationIo, schemars::JsonSchema))]
#[non_exhaustive]
pub struct HttpError {
    /// The HTTP status code for the error.
    ///
    /// When this error is converted to an HTTP response, this field is set as the HTTP response
    /// status header and omitted from the response body/payload.
    #[serde(skip)]
    pub status: StatusCode,
    /// Basic description of the error that occurred.
    // Todo: auto-redact sensitive data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Additional details for the error.
    // Todo: auto-redact sensitive data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// The original error. This can be logged to help with debugging, but it is omitted
    /// from the response body/payload to avoid exposing sensitive details from the stacktrace
    /// to the user.
    #[source]
    #[serde(skip)]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl Display for HttpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Http Error {} - {:?}", self.status, self.error)
    }
}

impl HttpError {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            error: None,
            details: None,
            source: None,
        }
    }

    /// Utility method to convert this [HttpError] into an [enum@Error].
    pub fn to_err(self) -> Error {
        self.into()
    }

    pub fn error(self, error: impl ToString) -> Self {
        Self {
            error: Some(error.to_string()),
            ..self
        }
    }

    pub fn details(self, details: impl ToString) -> Self {
        Self {
            details: Some(details.to_string()),
            ..self
        }
    }

    pub fn source(self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Some(Box::new(source)),
            ..self
        }
    }

    // Common 4xx errors

    /// Helper method to create an error with status code [StatusCode::BAD_REQUEST]
    pub fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST)
    }

    /// Helper method to create an error with status code [StatusCode::UNAUTHORIZED]
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED)
    }

    /// Helper method to create an error with status code [StatusCode::FORBIDDEN]
    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN)
    }

    /// Helper method to create an error with status code [StatusCode::NOT_FOUND]
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND)
    }

    /// Helper method to create an error with status code [StatusCode::GONE]
    pub fn gone() -> Self {
        Self::new(StatusCode::GONE)
    }

    // Common 5xx errors

    /// Helper method to create an error with status code [StatusCode::INTERNAL_SERVER_ERROR]
    pub fn internal_server_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl From<StatusCode> for HttpError {
    fn from(value: StatusCode) -> Self {
        HttpError::new(value)
    }
}

impl From<StatusCode> for Error {
    fn from(value: StatusCode) -> Self {
        HttpError::new(value).into()
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let status = self.status;
        let mut res = Json(self).into_response();
        *res.status_mut() = status;
        res
    }
}
