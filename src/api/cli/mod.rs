use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::app::context::AppContext;
use crate::app::App;
#[cfg(test)]
use crate::app::MockApp;
use crate::error::RoadsterResult;
use crate::migration::Migrator;
use crate::service::registry::ServiceRegistry;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::{Args, Command, FromArgMatches};
use std::ffi::OsString;

pub mod roadster;

#[non_exhaustive]
pub struct CliState<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Sync + 'static,
{
    pub roadster_cli: RoadsterCli,
    pub app_cli: A::Cli,
    pub app: A,
    pub state: S,
    #[cfg(feature = "db-sql")]
    pub migrators: Vec<Box<dyn Migrator<S>>>,
    pub service_registry: ServiceRegistry<A, S>,
}

/// Implement to enable Roadster to run your custom CLI commands.
#[async_trait]
pub trait RunCommand<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + Sync,
{
    /// Run the command.
    ///
    /// # Returns
    /// * `Ok(true)` - If the implementation handled the command and thus the app should end execution
    ///     after the command is complete.
    /// * `Ok(false)` - If the implementation did not handle the command and thus the app should
    ///     continue execution after the command is complete.
    /// * `Err(...)` - If the implementation experienced an error while handling the command. The
    ///     app should end execution after the command is complete.
    async fn run(&self, cli: &CliState<A, S>) -> RoadsterResult<bool>;
}

pub(crate) fn parse_cli<A, S, I, T>(args: I) -> RoadsterResult<(RoadsterCli, A::Cli)>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    // Build the CLI by augmenting a default Command with both the roadster and app-specific CLIs
    let cli = Command::default();
    // Add the roadster CLI. Save the shared attributes to use after adding the app-specific CLI
    let cli = RoadsterCli::augment_args(cli);
    let about = cli.get_about().cloned();
    let long_about = cli.get_long_about().cloned();
    let version = cli.get_version().map(|x| x.to_string());
    let long_version = cli.get_long_version().map(|x| x.to_string());
    // Add the app-specific CLI. This will override the shared attributes, so we need to
    // combine them with the roadster CLI attributes.
    let cli = A::Cli::augment_args(cli);
    let cli = if let Some((a, b)) = about.zip(cli.get_about().cloned()) {
        cli.about(format!("{a}\n\n{b}"))
    } else {
        cli
    };
    let cli = if let Some((a, b)) = long_about.zip(cli.get_long_about().cloned()) {
        cli.long_about(format!("{a}\n\n{b}"))
    } else {
        cli
    };
    let cli = if let Some((a, b)) = version.zip(cli.get_version().map(|x| x.to_string())) {
        cli.version(format!("roadster: {a}, app: {b}"))
    } else {
        cli
    };
    let cli = if let Some((a, b)) = long_version.zip(cli.get_long_version().map(|x| x.to_string()))
    {
        cli.long_version(format!("roadster: {a}\n\napp: {b}"))
    } else {
        cli
    };
    // Build each CLI from the CLI args
    let matches = cli.get_matches_from(args);
    let roadster_cli = RoadsterCli::from_arg_matches(&matches)?;
    let app_cli = A::Cli::from_arg_matches(&matches)?;
    Ok((roadster_cli, app_cli))
}

pub(crate) async fn handle_cli<A, S>(cli: &CliState<A, S>) -> RoadsterResult<bool>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    if cli.roadster_cli.run(cli).await? {
        return Ok(true);
    }
    if cli.app_cli.run(cli).await? {
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
pub struct TestCli<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    _state: std::marker::PhantomData<S>,
}

#[cfg(test)]
mockall::mock! {
    pub TestCli<S>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {}

    #[async_trait]
    impl<S> RunCommand<MockApp<S>, S> for TestCli<S>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        async fn run(&self, prepared: &CliState<MockApp<S>, S>) -> RoadsterResult<bool>;
    }

    impl<S> clap::FromArgMatches for TestCli<S>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error>;
        fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error>;
    }

    impl<S> clap::Args for TestCli<S>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        fn augment_args(cmd: clap::Command) -> clap::Command;
        fn augment_args_for_update(cmd: clap::Command) -> clap::Command;
    }

    impl<S> Clone for TestCli<S>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        fn clone(&self) -> Self;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockApp;
    use crate::service::registry::ServiceRegistry;
    use crate::testing::snapshot::TestCase;
    use insta::assert_toml_snapshot;
    use itertools::Itertools;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn setup_cli(
        args: Vec<&str>,
        mock_handles_cli: bool,
    ) -> (RoadsterCli, MockTestCli<AppContext>) {
        let augment_args_context = MockTestCli::<AppContext>::augment_args_context();
        augment_args_context.expect().returning(|c| c);
        let from_arg_matches_context = MockTestCli::<AppContext>::from_arg_matches_context();
        from_arg_matches_context
            .expect()
            .returning(|_| Ok(MockTestCli::<AppContext>::default()));

        let mut app_cli = MockTestCli::<AppContext>::default();
        app_cli
            .expect_run()
            .returning(move |_| Ok(mock_handles_cli));

        // The first word is interpreted as the binary name
        let args = vec!["binary_name"].into_iter().chain(args).collect_vec();

        let (roadster_cli, _) = super::parse_cli::<MockApp<AppContext>, _, _, _>(args).unwrap();

        (roadster_cli, app_cli)
    }

    #[rstest]
    #[case(None)]
    #[case(Some("--environment test"))]
    #[case(Some("--skip-validate-config"))]
    #[case(Some("--allow-dangerous"))]
    #[cfg_attr(feature = "open-api", case::list_routes(Some("roadster list-routes")))]
    #[cfg_attr(feature = "open-api", case::list_routes(Some("r list-routes")))]
    #[cfg_attr(feature = "open-api", case::open_api(Some("r open-api")))]
    #[cfg_attr(feature = "db-sql", case::migrate(Some("r migrate up")))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn parse_cli(_case: TestCase, #[case] args: Option<&str>) {
        // Arrange
        let args = args
            .map(|args| args.split(' ').collect_vec())
            .unwrap_or_default();

        // Act
        let (roadster_cli, _a) = setup_cli(args, false);

        // Assert
        assert_toml_snapshot!(roadster_cli);
    }

    #[rstest]
    #[case(None, false, false)]
    #[case(None, true, true)]
    #[cfg_attr(
        feature = "open-api",
        case::list_routes(Some("roadster handle-cli"), false, true)
    )]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn handle_cli(
        _case: TestCase,
        #[case] args: Option<&str>,
        #[case] mock_handles_cli: bool,
        #[case] cli_handled: bool,
    ) {
        let context = AppContext::test(None, None, None).unwrap();
        let app = MockApp::default();

        let args = args
            .map(|args| args.split(' ').collect_vec())
            .unwrap_or_default();

        let (roadster_cli, app_cli) = setup_cli(args, mock_handles_cli);

        let cli = CliState {
            roadster_cli,
            app_cli,
            app,
            #[cfg(feature = "db-sql")]
            migrators: Default::default(),
            service_registry: ServiceRegistry::new(&context),
            state: context,
        };

        let result = super::handle_cli(&cli).await.unwrap();

        assert_eq!(result, cli_handled);
    }
}
