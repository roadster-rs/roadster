use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::context::AppContext;
use crate::app::{App, PreparedApp};
use crate::error::RoadsterResult;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::{Parser, Subcommand};
use serde_derive::Serialize;
use tracing::warn;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct MigrateArgs {
    #[clap(subcommand)]
    pub command: MigrateCommand,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for MigrateArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, prepared_app: &PreparedApp<A, S>) -> RoadsterResult<bool> {
        self.command.run(prepared_app).await
    }
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum MigrateCommand {
    /// Apply pending migrations
    Up(UpArgs),
    /// Rollback applied migrations
    Down(DownArgs),
    /// Rollback all applied migrations, then reapply all migrations
    Refresh,
    /// Rollback all applied migrations
    Reset,
    /// Drop all tables from the database, then reapply all migrations
    Fresh,
    /// Check the status of all migrations
    Status,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for MigrateCommand
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, prepared_app: &PreparedApp<A, S>) -> RoadsterResult<bool> {
        let context = AppContext::from_ref(&prepared_app.state);
        if is_destructive(self) && !prepared_app.roadster_cli.allow_dangerous(&context) {
            return Err(anyhow!("Running destructive command `{:?}` is not allowed in environment `{:?}`. To override, provide the `--allow-dangerous` CLI arg.", self, context.config().environment).into());
        } else if is_destructive(self) {
            warn!(
                "Running destructive command `{:?}` in environment `{:?}`",
                self,
                context.config().environment
            );
        }
        match self {
            MigrateCommand::Up(args) => {
                for migrator in prepared_app.migrators.iter() {
                    migrator.up(&prepared_app.state).await?
                }
            }
            // MigrateCommand::Up(args) => A::M::up(state).await?,
            // MigrateCommand::Down(args) => A::M::down(context.db(), args.steps).await?,
            // MigrateCommand::Refresh => A::M::refresh(context.db()).await?,
            // MigrateCommand::Reset => A::M::reset(context.db()).await?,
            // MigrateCommand::Fresh => A::M::fresh(context.db()).await?,
            // MigrateCommand::Status => A::M::status(context.db()).await?,
            _ => unimplemented!(),
        };
        Ok(true)
    }
}

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct UpArgs {
    /// The number of pending migration steps to apply.
    #[clap(short = 'n', long)]
    pub steps: Option<u32>,
}

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct DownArgs {
    /// The number of applied migration steps to rollback.
    #[clap(short = 'n', long)]
    pub steps: Option<u32>,
}

fn is_destructive(command: &MigrateCommand) -> bool {
    !matches!(command, MigrateCommand::Status)
}
