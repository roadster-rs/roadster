use crate::app::App;
#[cfg(test)]
use crate::app::MockTestApp;
#[mockall_double::double]
use crate::app_context::AppContext;
use crate::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use async_trait::async_trait;
use clap::{Args, Command, FromArgMatches};

pub mod roadster;

/// Implement to enable Roadster to run your custom CLI commands.
#[async_trait]
pub trait RunCommand<A>
where
    A: App + ?Sized + Sync,
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
    async fn run(
        &self,
        app: &A,
        cli: &A::Cli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool>;
}

pub(crate) fn parse_cli<A>() -> anyhow::Result<(RoadsterCli, A::Cli)>
where
    A: App,
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
    let matches = cli.get_matches();
    let roadster_cli = RoadsterCli::from_arg_matches(&matches)?;
    let app_cli = A::Cli::from_arg_matches(&matches)?;
    Ok((roadster_cli, app_cli))
}

pub(crate) async fn handle_cli<A>(
    app: &A,
    roadster_cli: &RoadsterCli,
    app_cli: &A::Cli,
    context: &AppContext<A::State>,
) -> anyhow::Result<()>
where
    A: App,
{
    if roadster_cli.run(app, roadster_cli, context).await? {
        return Ok(());
    }
    if app_cli.run(app, app_cli, context).await? {
        return Ok(());
    }
    Ok(())
}

#[cfg(test)]
mockall::mock! {
    pub Cli {}

    #[async_trait]
    impl RunCommand<MockTestApp> for Cli {
        async fn run(
                &self,
                app: &MockTestApp,
                cli: &<MockTestApp as App>::Cli,
                context: &AppContext<<MockTestApp as App>::State>,
            ) -> anyhow::Result<bool>;
    }

    impl clap::FromArgMatches for Cli {
        fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error>;
        fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error>;
    }

    impl clap::Args for Cli {
        fn augment_args(cmd: clap::Command) -> clap::Command;
        fn augment_args_for_update(cmd: clap::Command) -> clap::Command;
    }
}
