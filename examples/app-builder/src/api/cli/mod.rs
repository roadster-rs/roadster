use crate::app_state::AppState;
use crate::App;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use roadster::api::cli::{CliState, RunCommand};
use roadster::error::RoadsterResult;

/// App builder example: Commands specific to managing the `app_builder` example app are provided in
/// the CLI as well. Subcommands not listed under the `roadster` subcommand are specific to
/// `app_builder`.
#[derive(Debug, Parser)]
#[command(version, about)]
#[non_exhaustive]
pub struct AppCli {
    #[command(subcommand)]
    pub command: Option<AppCommand>,
}

#[async_trait]
impl RunCommand<App, AppState> for AppCli {
    #[allow(clippy::disallowed_types)]
    async fn run(&self, prepared_app: &CliState<App, AppState>) -> RoadsterResult<bool> {
        if let Some(command) = self.command.as_ref() {
            command.run(prepared_app).await
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
impl RunCommand<App, AppState> for AppCommand {
    async fn run(&self, _prepared_app: &CliState<App, AppState>) -> RoadsterResult<bool> {
        Ok(false)
    }
}
