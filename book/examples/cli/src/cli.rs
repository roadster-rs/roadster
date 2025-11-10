use crate::App;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use roadster::api::cli::{CliState, RunCommand};
use roadster::app::context::AppContext;
use std::convert::Infallible;
use tracing::info;

/// CLI example: Commands specific to managing the `cli-example` app are provided in the CLI
/// as well. Subcommands not listed under the `roadster` subcommand are specific to `cli-example`.
#[derive(Debug, Parser)]
#[command(version, about)]
#[non_exhaustive]
pub struct AppCli {
    #[command(subcommand)]
    pub command: Option<AppCommand>,
}

#[async_trait]
impl RunCommand<App, AppContext> for AppCli {
    type Error = Infallible;

    #[allow(clippy::disallowed_types)]
    async fn run(&self, prepared_app: &CliState<App, AppContext>) -> Result<bool, Self::Error> {
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
pub enum AppCommand {
    /// Print a "hello world" message.
    HelloWorld,
}

#[async_trait]
impl RunCommand<App, AppContext> for AppCommand {
    type Error = Infallible;

    async fn run(&self, _prepared_app: &CliState<App, AppContext>) -> Result<bool, Self::Error> {
        match self {
            AppCommand::HelloWorld => {
                info!("Hello, world!");
            }
        }
        Ok(true)
    }
}
