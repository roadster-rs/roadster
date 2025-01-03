#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCli;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
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
pub mod worker;

/// Trait to represent a service (e.g., a persistent task) to run in the app. Example services
/// include, but are not limited to: an [http API][crate::service::http::service::HttpService],
/// a sidekiq processor, or a gRPC API.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AppService<A, S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    /// The name of the service.
    fn name(&self) -> String;

    /// Whether the service is enabled. If the service is not enabled, it will not be run.
    fn enabled(&self, state: &S) -> bool;

    /// Called when the app is starting up allow the service to handle CLI commands.
    ///
    /// Note: this is called after attempting to handle the CLI commands via [crate::api::cli::RunCommand]
    /// implementations and won't be called if the command was already handled.
    ///
    /// See [crate::api::cli::RunCommand] for an explanation of the return values -- the behavior is
    /// the same.
    #[cfg(feature = "cli")]
    async fn handle_cli(
        &self,
        _roadster_cli: &RoadsterCli,
        _app_cli: &A::Cli,
        _state: &S,
    ) -> RoadsterResult<bool> {
        Ok(false)
    }

    /// Perform any initialization work or other checks that should be done before the service runs.
    ///
    /// For example, checking that the service is healthy, removing stale items from the
    /// service's queue, etc.
    async fn before_run(&self, _state: &S) -> RoadsterResult<()> {
        Ok(())
    }

    /// Run the service in a new tokio task.
    ///
    /// * cancel_token - A tokio [CancellationToken] to use as a signal to gracefully shut down
    /// the service.
    async fn run(self: Box<Self>, state: &S, cancel_token: CancellationToken)
        -> RoadsterResult<()>;
}

/// Trait used to build an [AppService]. It's not a requirement that services implement this
/// trait; it is provided as a convenience. A [builder][AppServiceBuilder] can be provided to
/// the [ServiceRegistry][crate::service::registry::ServiceRegistry] instead of an [AppService],
/// in which case the [ServiceRegistry][crate::service::registry::ServiceRegistry] will only
/// build and register the service if [AppService::enabled] is `true`.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AppServiceBuilder<A, S, Service>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
    Service: AppService<A, S>,
{
    fn name(&self) -> String;

    fn enabled(&self, state: &S) -> bool;

    async fn build(self, state: &S) -> RoadsterResult<Service>;
}
