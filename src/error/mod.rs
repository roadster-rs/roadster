pub mod api;
pub mod auth;
#[cfg(feature = "http")]
pub mod axum;
pub mod config;
pub mod other;
pub mod serde;
#[cfg(feature = "sidekiq")]
pub mod sidekiq;
pub mod tokio;
#[cfg(feature = "grpc")]
pub mod tonic;
pub mod tracing;

use crate::error::api::ApiError;
use crate::error::auth::AuthError;
#[cfg(feature = "http")]
use crate::error::axum::AxumError;
use crate::error::other::OtherError;
use crate::error::serde::SerdeError;
#[cfg(feature = "sidekiq")]
use crate::error::sidekiq::SidekiqError;
use crate::error::tokio::TokioError;
#[cfg(feature = "grpc")]
use crate::error::tonic::TonicError;
use crate::error::tracing::TracingError;
#[cfg(feature = "http")]
use ::axum::http::StatusCode;
#[cfg(feature = "http")]
use ::axum::response::{IntoResponse, Response};
use ::tracing::debug;
#[cfg(feature = "open-api")]
use aide::OperationOutput;
use thiserror::Error;

pub type RoadsterResult<T> = Result<T, Error>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(transparent)]
    Auth(#[from] AuthError),

    #[error(transparent)]
    Serde(#[from] SerdeError),

    #[cfg(feature = "db-sql")]
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),

    #[cfg(feature = "sidekiq")]
    #[error(transparent)]
    Sidekiq(#[from] SidekiqError),

    #[cfg(feature = "cli")]
    #[error(transparent)]
    Clap(#[from] clap::error::Error),

    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Validation(#[from] validator::ValidationErrors),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Tokio(#[from] TokioError),

    #[error(transparent)]
    Tracing(#[from] TracingError),

    #[cfg(feature = "http")]
    #[error(transparent)]
    Axum(#[from] AxumError),

    #[cfg(feature = "grpc")]
    #[error(transparent)]
    Tonic(#[from] TonicError),

    #[error(transparent)]
    Other(#[from] OtherError),
}

#[cfg(feature = "http")]
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        debug!("{}", self);
        match self {
            Error::Api(err) => err.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[cfg(feature = "open-api")]
impl OperationOutput for Error {
    type Inner = api::http::HttpError;
}
