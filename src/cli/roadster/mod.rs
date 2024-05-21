use crate::app::App;
#[mockall_double::double]
use crate::app_context::AppContext;
#[cfg(feature = "open-api")]
use crate::cli::roadster::list_routes::ListRoutesArgs;
#[cfg(feature = "open-api")]
use crate::cli::roadster::migrate::MigrateArgs;
#[cfg(feature = "open-api")]
use crate::cli::roadster::open_api_schema::OpenApiArgs;
use crate::cli::roadster::print_config::PrintConfigArgs;
use crate::config::environment::Environment;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use serde_derive::Serialize;

#[cfg(feature = "open-api")]
pub mod list_routes;
#[cfg(feature = "db-sql")]
pub mod migrate;
#[cfg(feature = "open-api")]
pub mod open_api_schema;
pub mod print_config;

/// Internal version of [RunCommand][crate::cli::RunCommand] that uses the [RoadsterCli] and
/// [AppContext] instead of the consuming app's versions of these objects. This (slightly) reduces
/// the boilerplate required to implement a Roadster command.
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
#[derive(Debug, Parser, Serialize)]
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

#[derive(Debug, Subcommand, Serialize)]
#[serde(tag = "type")]
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

#[derive(Debug, Parser, Serialize)]
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

#[derive(Debug, Subcommand, Serialize)]
#[serde(tag = "type")]
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
