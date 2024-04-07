use async_trait::async_trait;
use clap::{Parser, Subcommand};

use crate::app_context::AppContext;
use crate::config::environment::Environment;

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

#[async_trait]
impl<S> RunCommand<RoadsterCli, S> for RoadsterCli
where
    S: Sync,
{
    async fn run(
        &self,
        cli: &RoadsterCli,
        context: &AppContext,
        state: &S,
    ) -> anyhow::Result<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(cli, context, state).await
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
impl<S> RunCommand<RoadsterCli, S> for RoadsterCommand
where
    S: Sync,
{
    async fn run(
        &self,
        cli: &RoadsterCli,
        context: &AppContext,
        state: &S,
    ) -> anyhow::Result<bool> {
        match self {
            RoadsterCommand::Roadster(args) => args.run(cli, context, state).await,
        }
    }
}

#[derive(Debug, Parser)]
pub struct RoadsterArgs {}

#[async_trait]
impl<S> RunCommand<RoadsterCli, S> for RoadsterArgs
where
    S: Sync,
{
    async fn run(
        &self,
        _cli: &RoadsterCli,
        _context: &AppContext,
        _state: &S,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }
}
