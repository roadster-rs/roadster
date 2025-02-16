use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::context::AppContext;
use crate::app::{App, PreparedApp};
use crate::error::RoadsterResult;
use crate::migration::{DownArgs, Migration, UpArgs};
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
            // Todo: reduce duplication
            MigrateCommand::Up(args) => {
                let mut total_steps_run = 0;
                for migrator in prepared_app.migrators.iter() {
                    let remaining_steps = args
                        .steps
                        .map(|steps| steps.saturating_sub(total_steps_run));
                    if let Some(remaining) = remaining_steps {
                        if remaining == 0 {
                            return Ok(true);
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
            }
            MigrateCommand::Down(args) => {
                let mut total_steps_run = 0;
                for migrator in prepared_app.migrators.iter().rev() {
                    let remaining_steps = args
                        .steps
                        .map(|steps| steps.saturating_sub(total_steps_run));
                    if let Some(remaining) = remaining_steps {
                        if remaining == 0 {
                            return Ok(true);
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
            }
            // MigrateCommand::Refresh => A::M::refresh(context.db()).await?,
            // MigrateCommand::Reset => A::M::reset(context.db()).await?,
            // MigrateCommand::Fresh => A::M::fresh(context.db()).await?,
            MigrateCommand::Status => {
                let mut migrations: Vec<Migration> = Vec::new();
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
            }
        };
        Ok(true)
    }
}

fn is_destructive(command: &MigrateCommand) -> bool {
    !matches!(command, MigrateCommand::Status)
}
