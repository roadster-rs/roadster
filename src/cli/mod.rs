use async_trait::async_trait;
use clap::{Parser, Subcommand};

use crate::app_context::AppContext;
#[cfg(feature = "open-api")]
use crate::cli::list_routes::ListRoutesArgs;
#[cfg(feature = "open-api")]
use crate::cli::open_api_schema::OpenApiArgs;
use crate::config::environment::Environment;

#[cfg(feature = "open-api")]
pub mod list_routes;
#[cfg(feature = "open-api")]
pub mod open_api_schema;

/// Implement to enable Roadster to run your custom CLI commands.
#[async_trait]
pub trait RunCommand<C, S>
where
    C: clap::Args,
    S: Sync,
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
    ///
    /// # Arguments
    ///
    /// * `cli` - The root-level clap args that were parsed, e.g. [RoadsterCli] or [crate::app::App::Cli].
    /// * `context` - The [context][AppContext] for the app.
    /// * `state` - The [state][crate::app::App::State] for the app.
    async fn run(&self, cli: &C, context: &AppContext, state: &S) -> anyhow::Result<bool>;
}

/// Specialized version of [RunCommand] that removes the `C` and `S` generics because we know what
/// `C` is and we don't need the custom app state `S` within roadster, so we don't need to provide
/// them everytime time we want to implement a roadster command.
#[async_trait]
trait RunRoadsterCommand {
    async fn run(&self, cli: &RoadsterCli, context: &AppContext) -> anyhow::Result<bool>;
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
    #[command(subcommand)]
    pub command: Option<RoadsterCommand>,
}

/// We implement [RunCommand] instead of [RunRoadsterCommand] for the top-level [RoadsterCli] so
/// we can run the roadster cli in the same way as the app-specific cli.
#[async_trait]
impl<S> RunCommand<RoadsterCli, S> for RoadsterCli
where
    S: Sync,
{
    async fn run(
        &self,
        cli: &RoadsterCli,
        context: &AppContext,
        _state: &S,
    ) -> anyhow::Result<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(cli, context).await
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
impl RunRoadsterCommand for RoadsterCommand {
    async fn run(&self, cli: &RoadsterCli, context: &AppContext) -> anyhow::Result<bool> {
        match self {
            RoadsterCommand::Roadster(args) => args.run(cli, context).await,
        }
    }
}

#[derive(Debug, Parser)]
pub struct RoadsterArgs {
    #[command(subcommand)]
    pub command: RoadsterSubCommand,
}

#[async_trait]
impl RunRoadsterCommand for RoadsterArgs {
    async fn run(&self, cli: &RoadsterCli, context: &AppContext) -> anyhow::Result<bool> {
        self.command.run(cli, context).await
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
}

#[async_trait]
impl RunRoadsterCommand for RoadsterSubCommand {
    async fn run(&self, cli: &RoadsterCli, context: &AppContext) -> anyhow::Result<bool> {
        match self {
            #[cfg(feature = "open-api")]
            RoadsterSubCommand::ListRoutes(args) => args.run(cli, context).await,
            #[cfg(feature = "open-api")]
            RoadsterSubCommand::OpenApi(args) => args.run(cli, context).await,
        }
    }
}
