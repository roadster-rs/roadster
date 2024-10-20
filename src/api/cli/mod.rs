use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::app::context::AppContext;
use crate::app::App;
#[cfg(test)]
use crate::app::MockApp;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::{Args, Command, FromArgMatches};
use std::ffi::OsString;

pub mod roadster;

/// Implement to enable Roadster to run your custom CLI commands.
#[async_trait]
pub trait RunCommand<A, S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + ?Sized + Sync,
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
    async fn run(&self, app: &A, cli: &A::Cli, state: &S) -> RoadsterResult<bool>;
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

pub(crate) async fn handle_cli<A, S>(
    app: &A,
    roadster_cli: &RoadsterCli,
    app_cli: &A::Cli,
    state: &S,
) -> RoadsterResult<bool>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    if roadster_cli.run(app, roadster_cli, state).await? {
        return Ok(true);
    }
    if app_cli.run(app, app_cli, state).await? {
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
        async fn run(&self, app: &MockApp<S>, cli: &<MockApp<S> as App<S>>::Cli, state: &S) -> RoadsterResult<bool>;
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
    use crate::testing::snapshot::TestCase;
    use insta::assert_toml_snapshot;
    use itertools::Itertools;
    use rstest::{fixture, rstest};

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(None, None)]
    #[case(Some("--environment test"), None)]
    #[case(Some("--skip-validate-config"), None)]
    #[case(Some("--allow-dangerous"), None)]
    #[cfg_attr(
        feature = "open-api",
        case::list_routes(Some("roadster list-routes"), None)
    )]
    #[cfg_attr(feature = "open-api", case::list_routes(Some("r list-routes"), None))]
    #[cfg_attr(feature = "open-api", case::open_api(Some("r open-api"), None))]
    #[cfg_attr(feature = "db-sql", case::migrate(Some("r migrate up"), None))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn parse_cli(_case: TestCase, #[case] args: Option<&str>, #[case] arg_list: Option<Vec<&str>>) {
        // Arrange
        let augment_args_context = MockTestCli::<AppContext>::augment_args_context();
        augment_args_context.expect().returning(|c| c);
        let from_arg_matches_context = MockTestCli::<AppContext>::from_arg_matches_context();
        from_arg_matches_context
            .expect()
            .returning(|_| Ok(MockTestCli::<AppContext>::default()));

        let args = if let Some(args) = args {
            args.split(' ').collect_vec()
        } else {
            arg_list.unwrap_or_default()
        };
        // The first word is interpreted as the binary name
        let args = vec!["binary_name"]
            .into_iter()
            .chain(args.into_iter())
            .collect_vec();

        // Act
        let (roadster_cli, _a) = super::parse_cli::<MockApp<AppContext>, _, _, _>(args).unwrap();

        // Assert
        assert_toml_snapshot!(roadster_cli);
    }
}
