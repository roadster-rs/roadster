pub mod api;
pub mod auth;
#[cfg(feature = "http")]
pub mod axum;
#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
#[cfg(feature = "db-sql")]
pub mod db;
#[cfg(feature = "email")]
pub mod email;
#[cfg(feature = "http")]
pub mod mime;
pub mod mutex;
pub mod other;
pub mod parse;
#[cfg(feature = "worker-pg")]
pub mod pgmq;
pub mod reqwest;
pub mod serde;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;
pub mod tokio;
#[cfg(feature = "grpc")]
pub mod tonic;
pub mod tracing;
#[cfg(feature = "worker")]
pub mod worker;

use crate::app::context::extension::ExtensionRegistryError;
use crate::error::api::ApiError;
use crate::error::auth::AuthError;
#[cfg(feature = "http")]
use crate::error::axum::AxumError;
#[cfg(feature = "cli")]
use crate::error::cli::CliError;
#[cfg(feature = "db-sql")]
use crate::error::db::DbError;
#[cfg(feature = "email")]
use crate::error::email::EmailError;
#[cfg(feature = "http")]
use crate::error::mime::MimeError;
use crate::error::mutex::MutexError;
use crate::error::other::OtherError;
use crate::error::parse::ParseError;
#[cfg(feature = "worker-pg")]
use crate::error::pgmq::PgmqError;
use crate::error::reqwest::ReqwestError;
use crate::error::serde::SerdeError;
#[cfg(feature = "worker-sidekiq")]
use crate::error::sidekiq::SidekiqError;
use crate::error::tokio::TokioError;
#[cfg(feature = "grpc")]
use crate::error::tonic::TonicError;
use crate::error::tracing::TracingError;
#[cfg(feature = "worker")]
use crate::error::worker::WorkerError;
use crate::health::check::registry::HealthCheckRegistryError;
use crate::lifecycle::registry::LifecycleHandlerRegistryError;
use crate::service::registry::ServiceRegistryError;
#[cfg(feature = "http")]
use ::axum::http::StatusCode;
#[cfg(feature = "http")]
use ::axum::response::{IntoResponse, Response};
#[cfg(feature = "open-api")]
use aide::OperationOutput;
use std::convert::Infallible;
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
    Db(#[from] DbError),

    #[cfg(feature = "worker-sidekiq")]
    #[error(transparent)]
    Sidekiq(#[from] SidekiqError),

    #[cfg(feature = "worker-pg")]
    #[error(transparent)]
    Pgmq(#[from] PgmqError),

    #[cfg(feature = "worker")]
    #[error(transparent)]
    Worker(#[from] WorkerError),

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

    #[error(transparent)]
    Reqwest(#[from] ReqwestError),

    #[cfg(feature = "http")]
    #[error(transparent)]
    Axum(#[from] AxumError),

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[cfg(feature = "http")]
    #[error(transparent)]
    Mime(#[from] MimeError),

    #[cfg(feature = "grpc")]
    #[error(transparent)]
    Tonic(#[from] TonicError),

    #[cfg(feature = "email")]
    #[error(transparent)]
    Email(#[from] EmailError),

    #[error(transparent)]
    HealthCheckRegistry(#[from] HealthCheckRegistryError),

    #[error(transparent)]
    LifecycleHandlerRegistry(#[from] LifecycleHandlerRegistryError),

    #[error(transparent)]
    ServiceRegistry(#[from] ServiceRegistryError),

    #[error(transparent)]
    ExtensionRegistry(#[from] ExtensionRegistryError),

    #[error(transparent)]
    Mutex(#[from] MutexError),

    #[error(transparent)]
    Infallible(#[from] Infallible),

    #[cfg(feature = "test-containers")]
    #[error(transparent)]
    TestContainers(
        #[from] testcontainers_modules::testcontainers::core::error::TestcontainersError,
    ),

    #[cfg(feature = "cli")]
    #[error(transparent)]
    Cli(#[from] CliError),

    #[error(transparent)]
    Other(#[from] OtherError),
}

#[cfg(feature = "http")]
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        ::tracing::debug!("{}", self);
        match self {
            Error::Api(err) => err.into_response(),
            Error::Auth(_) => StatusCode::UNAUTHORIZED.into_response(),
            _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[cfg(feature = "open-api")]
impl OperationOutput for Error {
    type Inner = api::http::HttpError;
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "http")]
    use axum_core::response::IntoResponse;

    #[rstest::fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> crate::testing::snapshot::TestCase {
        Default::default()
    }

    #[rstest::rstest]
    #[case(
        crate::error::api::http::HttpError::bad_request().to_err()
    )]
    #[case(
        crate::error::api::http::HttpError::unauthorized().to_err()
    )]
    #[case(
        crate::error::api::http::HttpError::forbidden().to_err()
    )]
    #[case(
        crate::error::api::http::HttpError::not_found().to_err()
    )]
    #[case(
        crate::error::api::http::HttpError::gone().to_err()
    )]
    #[case(
        crate::error::api::http::HttpError::internal_server_error().to_err()
    )]
    #[case(
        crate::error::api::http::HttpError::from(axum::http::StatusCode::BAD_REQUEST).into()
    )]
    #[case(
        axum::http::StatusCode::BAD_REQUEST.into()
    )]
    #[case(
        crate::error::api::ApiError::Other(Box::new(crate::error::other::OtherError::Message("error".to_owned()))).into()
    )]
    #[cfg(feature = "http")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn into_response(
        _case: crate::testing::snapshot::TestCase,
        #[case] error: crate::error::Error,
    ) {
        insta::assert_debug_snapshot!(error.into_response().status());
    }
}
