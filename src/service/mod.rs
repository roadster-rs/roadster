use crate::app::App;
use crate::app_context::AppContext;
#[cfg(feature = "cli")]
use crate::cli::RoadsterCli;
use async_trait::async_trait;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub mod http;
pub mod registry;

/// Trait to represent a service (e.g., a persistent task) to run in the app. Example services
/// include, but are not limited to: an [http API][crate::service::http::http_service::HttpService],
/// a sidekiq processor, or a gRPC API.
#[async_trait]
pub trait AppService<A: App>: Send + Sync {
    /// The name of the service.
    fn name() -> String
    where
        Self: Sized;

    /// Whether the service is enabled. If the service is not enabled, it will not be run.
    fn enabled(context: &AppContext, state: &A::State) -> bool
    where
        Self: Sized;

    /// Called when the app is starting up allow the service to handle CLI commands.
    ///
    /// Note: this is called after attempting to handle the CLI commands via [crate::cli::RunCommand]
    /// implementations and won't be called if the command was already handled.
    ///
    /// See [crate::cli::RunCommand] for an explanation of the return values -- the behavior is
    /// the same.
    #[cfg(feature = "cli")]
    async fn handle_cli(
        &self,
        _roadster_cli: &RoadsterCli,
        _app_cli: &A::Cli,
        _app_context: &AppContext,
        _app_state: &A::State,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// Run the service in a new tokio task.
    ///
    /// * cancel_token - A tokio [CancellationToken] to use as a signal to gracefully shut down
    /// the service.
    async fn run(
        &self,
        app_context: Arc<AppContext>,
        app_state: Arc<A::State>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()>;
}

/// Trait used to build an [AppService]. It's not a requirement that services implement this
/// trait; it is provided as a convenience. A [builder][AppServiceBuilder] can be provided to
/// the [ServiceRegistry][crate::service::registry::ServiceRegistry] instead of an [AppService],
/// in which case the [ServiceRegistry][crate::service::registry::ServiceRegistry] will only
/// build and register the service if [AppService::enabled] is `true`.
pub trait AppServiceBuilder<A, S>
where
    A: App,
    S: AppService<A>,
{
    fn build(self, context: &AppContext, state: &A::State) -> anyhow::Result<S>;
}
