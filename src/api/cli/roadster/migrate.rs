use anyhow::anyhow;
use async_trait::async_trait;

use axum_core::extract::FromRef;
use clap::{Parser, Subcommand};
use sea_orm_migration::MigratorTrait;
use serde_derive::Serialize;
use tracing::warn;

use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;

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
    async fn run(&self, app: &A, cli: &RoadsterCli, state: &S) -> RoadsterResult<bool> {
        self.command.run(app, cli, state).await
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
    async fn run(&self, _app: &A, cli: &RoadsterCli, state: &S) -> RoadsterResult<bool> {
        let context = AppContext::from_ref(state);
        if is_destructive(self) && !cli.allow_dangerous(&context) {
            return Err(anyhow!("Running destructive command `{:?}` is not allowed in environment `{:?}`. To override, provide the `--allow-dangerous` CLI arg.", self, context.config().environment).into());
        } else if is_destructive(self) {
            warn!(
                "Running destructive command `{:?}` in environment `{:?}`",
                self,
                context.config().environment
            );
        }
        match self {
            MigrateCommand::Up(args) => A::M::up(context.db(), args.steps).await?,
            MigrateCommand::Down(args) => A::M::down(context.db(), args.steps).await?,
            MigrateCommand::Refresh => A::M::refresh(context.db()).await?,
            MigrateCommand::Reset => A::M::reset(context.db()).await?,
            MigrateCommand::Fresh => A::M::fresh(context.db()).await?,
            MigrateCommand::Status => A::M::status(context.db()).await?,
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
