//! Mostly copied from <https://github.com/tamasfe/aide/blob/6a3ca0107409797baf31f0ebf30724b39e880f7e/examples/example-axum/src/errors.rs#L9>

#[cfg(feature = "open-api")]
use aide::OperationIo;
use axum::{http::StatusCode, response::IntoResponse};
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde::Serialize;
use serde_derive::Deserialize;
use serde_json::Value;
use std::fmt::Debug;
use tracing::error;

/// A default error response for most API errors.
/// Todo: Helpers for various status codes.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(OperationIo, JsonSchema))]
pub struct AppError {
    /// An error message.
    pub error: String,
    #[serde(skip)]
    pub status: StatusCode,
    /// Optional Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<Value>,
}

impl AppError {
    pub fn new(error: &str) -> Self {
        Self {
            error: error.to_string(),
            status: StatusCode::BAD_REQUEST,
            error_details: None,
        }
    }

    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.error_details = Some(details);
        self
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status;
        let mut res = axum::Json(self).into_response();
        *res.status_mut() = status;
        res
    }
}

/// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
/// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        // By default, we don't want to return the details from the anyhow::Error to the user,
        // so we just emit the error as a trace and return a generic error message.
        // Todo: Figure out a good way to return some details while ensuring we don't return
        //  any sensitive details.
        error!("{}", err.into());
        Self {
            error: "Something went wrong".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            error_details: None,
        }
    }
}
