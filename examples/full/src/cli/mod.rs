use async_trait::async_trait;
use clap::{Parser, Subcommand};
use roadster::app::context::AppContext;

use roadster::api::cli::RunCommand;
use roadster::error::RoadsterResult;

use crate::app::App;
use crate::app_state::CustomAppContext;

/// Full Example: Commands specific to managing the `full` app are provided in the CLI
/// as well. Subcommands not listed under the `roadster` subcommand are specific to `full`.
#[derive(Debug, Parser)]
#[command(version, about)]
#[non_exhaustive]
pub struct AppCli {
    #[command(subcommand)]
    pub command: Option<AppCommand>,
}

#[async_trait]
impl RunCommand<App> for AppCli {
    #[allow(clippy::disallowed_types)]
    async fn run(
        &self,
        app: &App,
        cli: &AppCli,
        context: &AppContext<CustomAppContext>,
    ) -> RoadsterResult<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(app, cli, context).await
        } else {
            Ok(false)
        }
    }
}

/// App specific subcommands
///
/// Note: This doc comment doesn't appear in the CLI `--help` message.
#[derive(Debug, Subcommand)]
pub enum AppCommand {}

#[async_trait]
impl RunCommand<App> for AppCommand {
    async fn run(
        &self,
        _app: &App,
        _cli: &AppCli,
        _context: &AppContext<CustomAppContext>,
    ) -> RoadsterResult<bool> {
        Ok(false)
    }
}
