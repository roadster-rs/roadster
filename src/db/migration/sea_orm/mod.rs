use crate::app::context::AppContext;
use crate::db::migration::{DownArgs, MigrationInfo, MigrationStatus, Migrator, UpArgs};
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use sea_orm_migration::MigratorTrait;
use std::cmp::min;
use std::marker::PhantomData;

pub mod check;
pub mod collation;
pub mod schema;
pub mod timestamp;
pub mod user;
pub mod uuid;

impl From<sea_orm_migration::Migration> for MigrationInfo {
    fn from(value: sea_orm_migration::Migration) -> Self {
        Self {
            name: value.name().to_string(),
            status: value.status().into(),
        }
    }
}

impl From<sea_orm_migration::MigrationStatus> for MigrationStatus {
    fn from(value: sea_orm_migration::MigrationStatus) -> Self {
        match value {
            sea_orm_migration::MigrationStatus::Applied => Self::Applied,
            sea_orm_migration::MigrationStatus::Pending => Self::Pending,
        }
    }
}

pub struct SeaOrmMigrator<M>
where
    M: MigratorTrait + Send + Sync,
{
    migrator: PhantomData<M>,
}

impl<M> SeaOrmMigrator<M>
where
    M: MigratorTrait + Send + Sync,
{
    pub fn new(_migrator: M) -> Self {
        Self {
            migrator: Default::default(),
        }
    }
}

#[async_trait]
impl<S, M> Migrator<S> for SeaOrmMigrator<M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: MigratorTrait + Send + Sync,
{
    #[tracing::instrument(skip_all)]
    async fn up(&self, state: &S, args: &UpArgs) -> crate::error::RoadsterResult<usize> {
        let context = crate::app::context::AppContext::from_ref(state);
        let pending = M::get_pending_migrations(context.sea_orm()).await?;

        let to_run = if let Some(steps) = args.steps {
            min(steps, pending.len())
        } else {
            pending.len()
        };

        M::up(context.sea_orm(), args.steps.map(|steps| steps as u32)).await?;

        // Assume all of the pending steps (up to `args.steps` count) ran successfully.
        Ok(to_run)
    }

    #[tracing::instrument(skip_all)]
    async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize> {
        let context = crate::app::context::AppContext::from_ref(state);
        let applied = M::get_applied_migrations(context.sea_orm()).await?;

        let to_roll_back = if let Some(steps) = args.steps {
            min(steps, applied.len())
        } else {
            applied.len()
        };

        M::down(context.sea_orm(), args.steps.map(|steps| steps as u32)).await?;

        // Assume all of the applied steps (up to `args.steps` count) were rolled back successfully.
        Ok(to_roll_back)
    }

    #[tracing::instrument(skip_all)]
    async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>> {
        let context = crate::app::context::AppContext::from_ref(state);

        let migrations = M::get_migration_with_status(context.sea_orm())
            .await?
            .into_iter()
            .map(|migration| migration.into())
            .collect();

        Ok(migrations)
    }
}
