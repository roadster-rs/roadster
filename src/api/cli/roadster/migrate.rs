use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::context::AppContext;
use crate::app::{App, PreparedApp};
use crate::error::RoadsterResult;
use crate::migration::{DownArgs, MigrationInfo, UpArgs};
use anyhow::anyhow;
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
    async fn run(&self, prepared_app: &PreparedApp<A, S>) -> RoadsterResult<bool> {
        self.command.run(prepared_app).await
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
    async fn run(&self, prepared_app: &PreparedApp<A, S>) -> RoadsterResult<bool> {
        let context = AppContext::from_ref(&prepared_app.state);
        // Todo: Refactor to allow `PreparedApp#cli` to not be an optional
        let allow_dangerous = prepared_app
            .cli
            .as_ref()
            .map(|cli| cli.roadster_cli.allow_dangerous(&context))
            .unwrap_or_default();

        if is_destructive(self) && !allow_dangerous {
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
                migrate_up(prepared_app, args).await?;
                print_status(prepared_app).await?;
            }
            MigrateCommand::Down(args) => {
                migrate_down(prepared_app, args).await?;
                print_status(prepared_app).await?;
            }
            MigrateCommand::Status => {
                print_status(prepared_app).await?;
            }
        };
        Ok(true)
    }
}

fn is_destructive(command: &MigrateCommand) -> bool {
    !matches!(command, MigrateCommand::Status)
}

// Todo: reduce duplication
async fn migrate_up<A, S>(prepared_app: &PreparedApp<A, S>, args: &UpArgs) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let mut total_steps_run = 0;
    for migrator in prepared_app.migrators.iter() {
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
                &prepared_app.state,
                &UpArgs::builder().steps_opt(remaining_steps).build(),
            )
            .await?;
        total_steps_run += steps_run;
    }
    Ok(())
}

// Todo: reduce duplication
async fn migrate_down<A, S>(prepared_app: &PreparedApp<A, S>, args: &DownArgs) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let mut total_steps_run = 0;
    for migrator in prepared_app.migrators.iter().rev() {
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
                &prepared_app.state,
                &DownArgs::builder().steps_opt(remaining_steps).build(),
            )
            .await?;
        total_steps_run += steps_run;
    }
    Ok(())
}

async fn print_status<A, S>(prepared_app: &PreparedApp<A, S>) -> RoadsterResult<()>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    let mut migrations: Vec<MigrationInfo> = Vec::new();
    for migrator in prepared_app.migrators.iter() {
        migrations.extend(migrator.status(&prepared_app.state).await?);
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
