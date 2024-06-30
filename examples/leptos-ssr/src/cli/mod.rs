use crate::app_state::AppState;
use crate::server::Server;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use roadster::api::cli::RunCommand;
use roadster::error::RoadsterResult;

/// Leptos SSR Example: Commands specific to managing the `leptos-ssr-example` app are provided in
/// the CLI as well. Subcommands not listed under the `roadster` subcommand are specific to
/// `leptos-ssr-example`.
#[derive(Debug, Parser)]
#[command(version, about)]
#[non_exhaustive]
pub struct AppCli {
    #[command(subcommand)]
    pub command: Option<AppCommand>,
}

#[async_trait]
impl RunCommand<Server, AppState> for AppCli {
    #[allow(clippy::disallowed_types)]
    async fn run(&self, app: &Server, cli: &AppCli, state: &AppState) -> RoadsterResult<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(app, cli, state).await
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
impl RunCommand<Server, AppState> for AppCommand {
    async fn run(&self, _app: &Server, _cli: &AppCli, _state: &AppState) -> RoadsterResult<bool> {
        Ok(false)
    }
}
