use anyhow::bail;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use sea_orm_migration::MigratorTrait;
use serde_derive::Serialize;
use tracing::warn;

use crate::app::App;
use crate::app_context::AppContext;
use crate::cli::roadster::{RoadsterCli, RunRoadsterCommand};

#[derive(Debug, Parser, Serialize)]
pub struct MigrateArgs {
    #[clap(subcommand)]
    pub command: MigrateCommand,
}

#[async_trait]
impl<A> RunRoadsterCommand<A> for MigrateArgs
where
    A: App,
{
    async fn run(
        &self,
        app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        self.command.run(app, cli, context).await
    }
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(tag = "type")]
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
impl<A> RunRoadsterCommand<A> for MigrateCommand
where
    A: App,
{
    async fn run(
        &self,
        _app: &A,
        cli: &RoadsterCli,
        context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        if is_destructive(self) && !cli.allow_dangerous(context) {
            bail!("Running destructive command `{:?}` is not allowed in environment `{:?}`. To override, provide the `--allow-dangerous` CLI arg.", self, context.config().environment);
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
pub struct UpArgs {
    /// The number of pending migration steps to apply.
    #[clap(short = 'n', long)]
    pub steps: Option<u32>,
}

#[derive(Debug, Parser, Serialize)]
pub struct DownArgs {
    /// The number of applied migration steps to rollback.
    #[clap(short = 'n', long)]
    pub steps: Option<u32>,
}

fn is_destructive(command: &MigrateCommand) -> bool {
    !matches!(command, MigrateCommand::Status)
}
