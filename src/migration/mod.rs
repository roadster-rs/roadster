//! This module provides pre-built SeaORM migrations for table schemas that are applicable
//! across many different applications and problem spaces.
//!
//! Additionally, some utilities are provided to create some common column types.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;

#[cfg(feature = "db-sea-orm")]
pub mod sea_orm;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Migrator<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    async fn up(&self, state: &S) -> RoadsterResult<()>;
}

#[cfg(feature = "db-diesel")]
#[async_trait]
impl<S> Migrator<S> for diesel_migrations::EmbeddedMigrations
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    #[tracing::instrument(skip_all)]
    async fn up(&self, state: &S) -> RoadsterResult<()> {
        use diesel::Connection;
        use diesel_migrations::MigrationHarness;

        tracing::info!("Starting migration");

        let migration_wrapper = EmbeddedMigrationsWrapper::try_from(self)?;

        // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
        //  aren't satisfied by the pooled connections currently, at least not for async
        //  connection pools.
        let context = AppContext::from_ref(state);
        let mut conn = diesel::PgConnection::establish(context.config().database.uri.as_ref())?;
        conn.run_pending_migrations(migration_wrapper)?;

        tracing::info!("Migrations completed");

        Ok(())
    }
}

/// [`MigrationHarness#run_pending_migrations`] takes an owned version of the
/// [`diesel_migrations::MigrationSource`], but our [`Migrator`] trait uses a reference. Because
/// [`diesel_migrations::EmbeddedMigrations`] doesn't implement [`Clone`], we can't directly use
/// it in our [`Migrator`]. However, [`diesel::migration::MigrationSource#migrations`] does take a
/// reference, so we can wrap it, pre-fetch the list of migrations, and then return them from the
/// wrapper's impl.
#[cfg(feature = "db-diesel")]
struct EmbeddedMigrationsWrapper<DB: diesel::backend::Backend> {
    migrations: std::sync::Mutex<Option<Vec<Box<dyn diesel::migration::Migration<DB>>>>>,
}

#[cfg(feature = "db-diesel")]
impl<DB: diesel::backend::Backend> TryFrom<&diesel_migrations::EmbeddedMigrations>
    for EmbeddedMigrationsWrapper<DB>
{
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(value: &diesel_migrations::EmbeddedMigrations) -> Result<Self, Self::Error> {
        Ok(Self {
            migrations: std::sync::Mutex::new(Some(
                <diesel_migrations::EmbeddedMigrations as diesel::migration::MigrationSource<
                    DB,
                >>::migrations(value)?,
            )),
        })
    }
}

#[cfg(feature = "db-diesel")]
impl<DB: diesel::backend::Backend> diesel::migration::MigrationSource<DB>
    for EmbeddedMigrationsWrapper<DB>
{
    fn migrations(
        &self,
    ) -> diesel::migration::Result<Vec<Box<dyn diesel::migration::Migration<DB>>>> {
        // We need to return an owned version of the migrations, and `diesel::migration::Migration`
        // doesn't implement `Clone`, so we put the migrations in a `Mutex<Option<...>>`, and
        // take the migrations out of the `Option`.

        let mut migrations = self
            .migrations
            .lock()
            // todo: poison error enum variant
            .map_err(|err| crate::error::Error::from(anyhow::anyhow!("{err}")))?;

        match migrations.take() {
            Some(migrations) => Ok(migrations),
            None => Ok(vec![]),
        }
    }
}
