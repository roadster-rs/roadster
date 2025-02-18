use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::migration::{DownArgs, MigrationInfo, MigrationStatus, Migrator, UpArgs};
use axum_core::extract::FromRef;
use diesel::backend::Backend;
use diesel::migration::{Migration, MigrationSource};
use diesel::Connection;
use diesel_migrations::{MigrationError, MigrationHarness};
use itertools::Itertools;
use std::cmp::min;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Mutex;
use tracing::info;

pub struct DieselMigrator<C>
where
    C: Send + Connection + MigrationHarness<C::Backend>,
{
    migrator: Box<dyn MigrationSource<C::Backend> + Send + Sync>,
    // Diesel connections don't implement `Sync`, so we need to wrap the `PhantomData` in a
    // `Mutex` to satisfy `Sync` trait bounds elsewhere.
    // https://github.com/diesel-rs/diesel/issues/190
    _conn: PhantomData<Mutex<C>>,
}

impl<C> DieselMigrator<C>
where
    C: Connection + Send + MigrationHarness<C::Backend>,
{
    pub fn new(migrator: impl 'static + MigrationSource<C::Backend> + Send + Sync) -> Self {
        Self {
            migrator: Box::new(migrator),
            _conn: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl<S, C> Migrator<S> for DieselMigrator<C>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    C: Connection + Send + MigrationHarness<C::Backend>,
{
    #[tracing::instrument(skip_all)]
    async fn up(&self, state: &S, args: &UpArgs) -> RoadsterResult<usize> {
        info!("Started applying migrations");

        let context = AppContext::from_ref(state);

        // Todo: use db pool from app state? May be able to use `AsyncConnectionWrapper` via
        //  `Deref`/`DerefMut` which is supposed to allow using it in an async context.
        //  See: https://github.com/weiznich/diesel_async/blob/main/CHANGELOG.md#051---2024-11-01
        let mut conn: C = Connection::establish(context.config().database.uri.as_ref())?;
        let pending =
            conn.pending_migrations(DieselMigrationSourceWrapper::try_from(&self.migrator)?)?;
        let pending = if let Some(steps) = args.steps {
            let steps = min(steps, pending.len());
            pending.into_iter().take(steps).collect()
        } else {
            pending
        };

        let completed = conn.run_migrations(&pending)?;
        let completed = completed.len();

        info!(count = completed, "Completed applying migrations");

        Ok(completed)
    }

    #[tracing::instrument(skip_all)]
    async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize> {
        info!("Started rolling back migrations");

        let context = AppContext::from_ref(state);
        let mut conn: C = Connection::establish(context.config().database.uri.as_ref())?;

        let to_roll_back = conn.applied_migrations()?.len();
        let to_roll_back = if let Some(steps) = args.steps {
            min(steps, to_roll_back)
        } else {
            to_roll_back
        };

        // This is mostly copied from the default `MigrationHarness#revert_all_migrations`
        // implementation, with a slight modification to only revert the first `to_roll_back`
        // migrations.
        let applied_versions = conn
            .applied_migrations()?
            .into_iter()
            .take(to_roll_back)
            .collect_vec();
        let mut migrations: HashMap<_, _> = self
            .migrator
            .migrations()?
            .into_iter()
            .map(|m| (m.name().version().as_owned(), m))
            .collect();

        for version in applied_versions {
            let migration_to_revert = migrations
                .remove(&version)
                .ok_or(MigrationError::UnknownMigrationVersion(version))?;
            info!(name=%migration_to_revert.name(), "Rolling back migration");
            conn.revert_migration(&migration_to_revert)?;
        }

        info!(count = to_roll_back, "Completed rolling back migrations");

        Ok(to_roll_back)
    }

    #[tracing::instrument(skip_all)]
    async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>> {
        let context = AppContext::from_ref(state);
        let mut conn: C = Connection::establish(context.config().database.uri.as_ref())?;

        let pending = conn
            .pending_migrations(DieselMigrationSourceWrapper::try_from(&self.migrator)?)?
            .into_iter()
            .map(|migration| {
                MigrationInfo::builder()
                    .name(migration.name().to_string())
                    .status(MigrationStatus::Pending)
                    .build()
            });

        let migrations: HashMap<_, _> = self
            .migrator
            .migrations()?
            .into_iter()
            .map(|m: Box<dyn Migration<C::Backend>>| (m.name().version().as_owned(), m))
            .collect();

        let applied = conn
            .applied_migrations()?
            .into_iter()
            .map(|version| {
                let name = migrations
                    .get(&version)
                    .map(|migration| migration.name().to_string())
                    .unwrap_or(version.to_string());
                MigrationInfo::builder()
                    .name(name)
                    .status(MigrationStatus::Applied)
                    .build()
            })
            .rev();

        let migrations = applied.into_iter().chain(pending.into_iter()).collect();

        Ok(migrations)
    }
}

/// [`MigrationHarness#run_pending_migrations`] takes an owned version of the
/// [`diesel_migrations::MigrationSource`], but our [`Migrator`] trait uses a reference. Because
/// [`diesel_migrations::EmbeddedMigrations`] doesn't implement [`Clone`], we can't directly use
/// it in our [`Migrator`]. However, [`MigrationSource#migrations`] does take a
/// reference, so we can wrap it, pre-fetch the list of migrations, and then return them from the
/// wrapper's impl.
struct DieselMigrationSourceWrapper<DB: Backend> {
    migrations: Mutex<Option<Vec<Box<dyn Migration<DB>>>>>,
}

impl<DB> TryFrom<&Box<dyn MigrationSource<DB> + Send + Sync>> for DieselMigrationSourceWrapper<DB>
where
    DB: Backend,
{
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(value: &Box<dyn MigrationSource<DB> + Send + Sync>) -> Result<Self, Self::Error> {
        Ok(Self {
            migrations: Mutex::new(Some(value.migrations()?)),
        })
    }
}

impl<DB: Backend> MigrationSource<DB> for DieselMigrationSourceWrapper<DB> {
    fn migrations(&self) -> diesel::migration::Result<Vec<Box<dyn Migration<DB>>>> {
        // We need to return an owned version of the migrations, and `Migration`
        // doesn't implement `Clone`, so we put the migrations in a `Mutex<Option<...>>`, and
        // take the migrations out of the `Option`.

        let mut migrations = self.migrations.lock().map_err(crate::error::Error::from)?;

        match migrations.take() {
            Some(migrations) => Ok(migrations),
            None => Err(anyhow::anyhow!("DieselMigrationSourceWrapper#migrations was called twice! This is not supported as the migrations were removed by the first call.").into()),
        }
    }
}
