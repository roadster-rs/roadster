use async_trait::async_trait;
use clap::{Parser, Subcommand};

use roadster::app_context::AppContext;
use roadster::cli::RunCommand;

use crate::app_state::AppState;

/// Custom version of [RunCommand] that removes the `C` and `S` generics because we know what they
/// are so we don't need to provide them every time we want to implement a command.
#[async_trait]
pub trait RunAppCommand {
    async fn run(&self, cli: &AppCli, state: &AppState) -> anyhow::Result<bool>;
}

/// Minimal Example: Commands specific to managing the `minimal` app are provided in the CLI
/// as well. Subcommands not listed under the `roadster` subcommand are specific to `minimal`.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct AppCli {
    #[command(subcommand)]
    pub command: Option<AppCommand>,
}

#[async_trait]
impl RunCommand<AppCli, AppState> for AppCli {
    #[allow(clippy::disallowed_types)]
    async fn run(
        &self,
        cli: &AppCli,
        _context: &AppContext,
        state: &AppState,
    ) -> anyhow::Result<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(cli, state).await
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
impl RunAppCommand for AppCommand {
    async fn run(&self, _cli: &AppCli, _state: &AppState) -> anyhow::Result<bool> {
        Ok(false)
    }
}
