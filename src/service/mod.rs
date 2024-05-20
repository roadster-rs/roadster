use crate::app::App;
#[mockall_double::double]
use crate::app_context::AppContext;
#[cfg(feature = "cli")]
use crate::cli::roadster::RoadsterCli;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "http")]
pub mod http;
pub mod registry;
pub(crate) mod runner;
pub mod worker;

/// Trait to represent a service (e.g., a persistent task) to run in the app. Example services
/// include, but are not limited to: an [http API][crate::service::http::service::HttpService],
/// a sidekiq processor, or a gRPC API.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait AppService<A: App + 'static>: Send + Sync {
    /// The name of the service.
    // todo: make this non-static? This would make some testing/mocking slightly easier
    fn name() -> String
    where
        Self: Sized;

    /// Whether the service is enabled. If the service is not enabled, it will not be run.
    // todo: make this non-static? This would make some testing/mocking slightly easier
    fn enabled(context: &AppContext<A::State>) -> bool
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
        _app_context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// Run the service in a new tokio task.
    ///
    /// * cancel_token - A tokio [CancellationToken] to use as a signal to gracefully shut down
    /// the service.
    async fn run(
        &self,
        app_context: &AppContext<A::State>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()>;
}

/// Trait used to build an [AppService]. It's not a requirement that services implement this
/// trait; it is provided as a convenience. A [builder][AppServiceBuilder] can be provided to
/// the [ServiceRegistry][crate::service::registry::ServiceRegistry] instead of an [AppService],
/// in which case the [ServiceRegistry][crate::service::registry::ServiceRegistry] will only
/// build and register the service if [AppService::enabled] is `true`.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait AppServiceBuilder<A, S>
where
    A: App + 'static,
    S: AppService<A>,
{
    fn enabled(&self, app_context: &AppContext<A::State>) -> bool {
        S::enabled(app_context)
    }

    async fn build(self, context: &AppContext<A::State>) -> anyhow::Result<S>;
}

#[cfg(test)]
mod tests {
    use crate::app::MockTestApp;
    use crate::app_context::MockAppContext;
    use crate::service::{AppServiceBuilder, MockAppService};
    use async_trait::async_trait;
    use rstest::rstest;

    struct TestAppServiceBuilder;
    #[async_trait]
    impl AppServiceBuilder<MockTestApp, MockAppService<MockTestApp>> for TestAppServiceBuilder {
        #[cfg_attr(coverage_nightly, coverage(off))]
        async fn build(
            self,
            _context: &MockAppContext<()>,
        ) -> anyhow::Result<MockAppService<MockTestApp>> {
            Ok(MockAppService::default())
        }
    }

    #[rstest]
    #[case(true)]
    #[case(false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_enabled(#[case] service_enabled: bool) {
        // Arrange
        let mut context = MockAppContext::default();
        context.expect_clone().returning(MockAppContext::default);

        let enabled_ctx = MockAppService::<MockTestApp>::enabled_context();
        enabled_ctx.expect().returning(move |_| service_enabled);

        // Act
        let builder = TestAppServiceBuilder;

        // Assert
        assert_eq!(builder.enabled(&context), service_enabled);
    }
}
