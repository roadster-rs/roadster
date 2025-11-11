use crate::app::context::AppContext;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use tokio_util::sync::CancellationToken;

pub mod function;
#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "http")]
pub mod http;
pub mod registry;
pub(crate) mod runner;
#[cfg(feature = "worker")]
pub mod worker;

/// Trait to represent a service (e.g., a persistent task) to run in the app.
///
/// # Examples
#[cfg_attr(
    feature = "http",
    doc = r"- [HTTP API][crate::service::http::service::HttpService]"
)]
#[cfg_attr(
    feature = "worker-sidekiq",
    doc = r"- [Sidekiq worker processor][crate::service::worker::backend::sidekiq::SidekiqWorkerService]"
)]
#[cfg_attr(
    feature = "worker-pg",
    doc = r"- [Postgres worker processor][crate::service::worker::backend::pg::PgWorkerService]"
)]
#[cfg_attr(
    feature = "grpc",
    doc = r"- [gRPC API][crate::service::grpc::service::GrpcService]"
)]
#[cfg_attr(test, mockall::automock(type Error = crate::error::Error;))]
#[async_trait]
pub trait Service<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    type Error: std::error::Error + Send + Sync;

    /// The name of the service.
    fn name(&self) -> String;

    /// Whether the service is enabled. If the service is not enabled, it will not be run.
    fn enabled(&self, state: &S) -> bool;

    /// Perform any initialization work or other checks that should be done before the service runs.
    ///
    /// For example, checking that the service is healthy, removing stale items from the
    /// service's queue, etc.
    ///
    /// Note that this is run for every service that's registered in the
    /// [`crate::service::registry::ServiceRegistry`] regardless of whether it's enabled or not.
    async fn before_run(&self, #[allow(unused_variables)] state: &S) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Run the service in a new tokio task.
    ///
    /// * cancel_token - A tokio [`CancellationToken`] to use as a signal to gracefully shut down
    /// the service.
    async fn run(
        self: Box<Self>,
        state: &S,
        cancel_token: CancellationToken,
    ) -> Result<(), Self::Error>;
}

/// Trait used to build a [`Service`]. It's not a requirement that services implement this
/// trait; it is provided as a convenience. A [builder][ServiceBuilder] can be provided to
/// the [`ServiceRegistry`][crate::service::registry::ServiceRegistry] instead of a [`Service`],
/// in which case the [`ServiceRegistry`][crate::service::registry::ServiceRegistry] will only
/// build and register the service if [`Service::enabled`] is `true`.
#[cfg_attr(test, mockall::automock(type Error = crate::error::Error;))]
#[async_trait]
pub trait ServiceBuilder<S, Srvc>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Srvc: Service<S>,
{
    type Error: Send + Sync + std::error::Error;

    fn name(&self) -> String;

    fn enabled(&self, state: &S) -> bool;

    async fn build(self, state: &S) -> Result<Srvc, Self::Error>;
}
