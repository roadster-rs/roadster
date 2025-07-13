use crate::api::cli::CliState;
use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::App;
use crate::app::context::AppContext;
use crate::db::migration::{DownArgs, MigrationInfo, UpArgs};
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use serde_derive::Serialize;
use tracing::{info, warn};

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
    async fn run(&self, cli: &CliState<A, S>) -> RoadsterResult<bool> {
        self.command.run(cli).await
    }
}

#[derive(Debug, Subcommand, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum MigrateCommand {
    /// Apply pending migrations. If no `steps` argument is provided, will apply all pending
    /// migrations.
    Up(UpArgs),
    /// Roll back applied migrations. If no `steps` argument is provided, will roll back all
    /// applied migrations.
    Down(DownArgs),
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
    async fn run(&self, cli: &CliState<A, S>) -> RoadsterResult<bool> {
        let context = AppContext::from_ref(&cli.state);

        let allow_dangerous = cli.roadster_cli.allow_dangerous(&context);

        if is_destructive(self) && !allow_dangerous {
            return Err(crate::error::cli::CliError::DestructiveCmdNotAllowed(
                context.config().environment.clone(),
            )
            .into());
        } else if is_destructive(self) {
            warn!(
                "Running destructive command `{:?}` in environment `{:?}`",
                self,
                context.config().environment
            );
        }

        match self {
            MigrateCommand::Up(args) => {
                migrate_up(cli, args).await?;
                print_status(cli).await?;
            }
            MigrateCommand::Down(args) => {
                migrate_down(cli, args).await?;
                print_status(cli).await?;
            }
            MigrateCommand::Status => {
                print_status(cli).await?;
            }
        };
        Ok(true)
    }
}

fn is_destructive(command: &MigrateCommand) -> bool {
    !matches!(command, MigrateCommand::Status)
}

// Todo: reduce duplication
async fn migrate_up<A, S>(cli: &CliState<A, S>, args: &UpArgs) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let mut total_steps_run = 0;
    for migrator in cli.migrators.iter() {
        let remaining_steps = args
            .steps
            .map(|steps| steps.saturating_sub(total_steps_run));
        if let Some(remaining) = remaining_steps {
            if remaining == 0 {
                return Ok(());
            }
        }
        let steps_run = migrator
            .up(
                &cli.state,
                &UpArgs::builder().maybe_steps(remaining_steps).build(),
            )
            .await?;
        total_steps_run += steps_run;
    }
    Ok(())
}

// Todo: reduce duplication
async fn migrate_down<A, S>(cli: &CliState<A, S>, args: &DownArgs) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let mut total_steps_run = 0;
    for migrator in cli.migrators.iter().rev() {
        let remaining_steps = args
            .steps
            .map(|steps| steps.saturating_sub(total_steps_run));
        if let Some(remaining) = remaining_steps {
            if remaining == 0 {
                return Ok(());
            }
        }
        let steps_run = migrator
            .down(
                &cli.state,
                &DownArgs::builder().maybe_steps(remaining_steps).build(),
            )
            .await?;
        total_steps_run += steps_run;
    }
    Ok(())
}

async fn print_status<A, S>(cli: &CliState<A, S>) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let mut migrations: Vec<MigrationInfo> = Vec::new();
    for migrator in cli.migrators.iter() {
        migrations.extend(migrator.status(&cli.state).await?);
    }
    let migrations = migrations
        .into_iter()
        .map(|migration| {
            let status: &'static str = migration.status.into();
            format!("{}\t{}", status, migration.name)
        })
        .join("\n");
    info!("Migration status:\n{migrations}");
    Ok(())
}
