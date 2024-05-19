use crate::app::App;
#[cfg(test)]
use crate::app::MockTestApp;
#[mockall_double::double]
use crate::app_context::AppContext;
#[cfg(feature = "open-api")]
use crate::cli::list_routes::ListRoutesArgs;
#[cfg(feature = "db-sql")]
use crate::cli::migrate::MigrateArgs;
#[cfg(feature = "open-api")]
use crate::cli::open_api_schema::OpenApiArgs;
use crate::cli::print_config::PrintConfigArgs;
use crate::config::environment::Environment;
use async_trait::async_trait;
use clap::{Parser, Subcommand};

#[cfg(feature = "open-api")]
pub mod list_routes;
#[cfg(feature = "db-sql")]
pub mod migrate;
#[cfg(feature = "open-api")]
pub mod open_api_schema;
pub mod print_config;

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

/// Internal version of [RunCommand] that uses the [RoadsterCli] and [AppContext] instead of
/// the consuming app's versions of these objects. This (slightly) reduces the boilerplate
/// required to implement a Roadster command.
#[async_trait]
pub(crate) trait RunRoadsterCommand<A>
where
    A: App,
{
    async fn run(
        &self,
        app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool>;
}

/// Roadster: The Roadster CLI provides various utilities for managing your application. If no subcommand
/// is matched, Roadster will default to running/serving your application.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct RoadsterCli {
    /// Specify the environment to use to run the application. This overrides the corresponding
    /// environment variable if it's set.
    #[clap(short, long)]
    pub environment: Option<Environment>,

    /// Skip validation of the app config. This can be useful for debugging the app config
    /// when used in conjunction with the `print-config` command.
    #[clap(long, action)]
    pub skip_validate_config: bool,

    /// Allow dangerous/destructive operations when running in the `production` environment. If
    /// this argument is not provided, dangerous/destructive operations will not be performed
    /// when running in `production`.
    #[clap(long, action)]
    pub allow_dangerous: bool,

    #[command(subcommand)]
    pub command: Option<RoadsterCommand>,
}

impl RoadsterCli {
    pub fn allow_dangerous<S>(&self, context: &AppContext<S>) -> bool {
        context.config().environment != Environment::Production || self.allow_dangerous
    }
}

#[async_trait]
impl<A> RunRoadsterCommand<A> for RoadsterCli
where
    A: App,
{
    async fn run(
        &self,
        app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(app, cli, context).await
        } else {
            Ok(false)
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum RoadsterCommand {
    /// Roadster subcommands. Subcommands provided by Roadster are listed under this subcommand in
    /// order to avoid naming conflicts with the consumer's subcommands.
    #[clap(visible_alias = "r")]
    Roadster(RoadsterArgs),
}

#[async_trait]
impl<A> RunRoadsterCommand<A> for RoadsterCommand
where
    A: App,
{
    async fn run(
        &self,
        app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        match self {
            RoadsterCommand::Roadster(args) => args.run(app, cli, context).await,
        }
    }
}

#[derive(Debug, Parser)]
pub struct RoadsterArgs {
    #[command(subcommand)]
    pub command: RoadsterSubCommand,
}

#[async_trait]
impl<A> RunRoadsterCommand<A> for RoadsterArgs
where
    A: App,
{
    async fn run(
        &self,
        app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        self.command.run(app, cli, context).await
    }
}

#[derive(Debug, Subcommand)]
pub enum RoadsterSubCommand {
    /// List the API routes available in the app. Note: only the routes defined
    /// using the `Aide` crate will be included in the output.
    #[cfg(feature = "open-api")]
    ListRoutes(ListRoutesArgs),

    /// Generate an OpenAPI 3.1 schema for the app's API routes. Note: only the routes defined
    /// using the `Aide` crate will be included in the schema.
    #[cfg(feature = "open-api")]
    OpenApi(OpenApiArgs),

    /// Perform DB operations using SeaORM migrations.
    #[cfg(feature = "db-sql")]
    #[clap(visible_aliases = ["m", "migration"])]
    Migrate(MigrateArgs),

    /// Print the AppConfig
    PrintConfig(PrintConfigArgs),
}

#[async_trait]
impl<A> RunRoadsterCommand<A> for RoadsterSubCommand
where
    A: App,
{
    async fn run(
        &self,
        app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        match self {
            #[cfg(feature = "open-api")]
            RoadsterSubCommand::ListRoutes(_) => {
                #[allow(unused_doc_comments)]
                /// Implemented by [crate::service::http::service::HttpService]
                Ok(false)
            }
            #[cfg(feature = "open-api")]
            RoadsterSubCommand::OpenApi(_) => {
                #[allow(unused_doc_comments)]
                /// Implemented by [crate::service::http::service::HttpService]
                Ok(false)
            }
            #[cfg(feature = "db-sql")]
            RoadsterSubCommand::Migrate(args) => args.run(app, cli, context).await,
            RoadsterSubCommand::PrintConfig(args) => args.run(app, cli, context).await,
        }
    }
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
