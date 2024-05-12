use async_trait::async_trait;
use clap::{Parser, Subcommand};
use roadster::app_context::AppContext;

use roadster::cli::RunCommand;

use crate::app::App;
use crate::app_state::AppState;

/// Minimal Example: Commands specific to managing the `minimal` app are provided in the CLI
/// as well. Subcommands not listed under the `roadster` subcommand are specific to `minimal`.
#[derive(Debug, Parser)]
#[command(version, about)]
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
        context: &AppContext<AppState>,
    ) -> anyhow::Result<bool> {
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
        _context: &AppContext<AppState>,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }
}
