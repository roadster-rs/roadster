//! This module provides pre-built SeaORM migrations for table schemas that are applicable
//! across many different applications and problem spaces.
//!
//! Additionally, some utilities are provided to create some common column types.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
#[cfg(feature = "db-diesel")]
use diesel::prelude::*;
use serde_derive::Serialize;
use std::marker::PhantomData;
use std::sync::Mutex;
use strum_macros::{EnumString, IntoStaticStr};
use typed_builder::TypedBuilder;

#[cfg(feature = "db-sea-orm")]
pub mod sea_orm;

#[derive(Debug, Serialize, TypedBuilder)]
#[cfg_attr(feature = "cli", derive(clap::Parser))]
#[non_exhaustive]
pub struct UpArgs {
    /// The number of pending migration steps to apply.
    #[cfg_attr(feature = "cli", clap(short = 'n', long))]
    #[builder(default, setter(strip_option(fallback = steps_opt)))]
    pub steps: Option<usize>,
}

#[derive(Debug, Serialize, TypedBuilder)]
#[cfg_attr(feature = "cli", derive(clap::Parser))]
#[non_exhaustive]
pub struct DownArgs {
    /// The number of applied migration steps to roll back.
    #[cfg_attr(feature = "cli", clap(short = 'n', long))]
    #[builder(default, setter(strip_option(fallback = steps_opt)))]
    pub steps: Option<usize>,
}

#[derive(Debug, Serialize, TypedBuilder)]
pub struct MigrationInfo {
    pub name: String,
    pub status: MigrationStatus,
}

#[cfg(feature = "db-sea-orm")]
impl From<sea_orm_migration::Migration> for MigrationInfo {
    fn from(value: sea_orm_migration::Migration) -> Self {
        Self {
            name: value.name().to_string(),
            status: value.status().into(),
        }
    }
}

#[derive(Debug, Serialize, EnumString, IntoStaticStr)]
pub enum MigrationStatus {
    Applied,
    Pending,
}

#[cfg(feature = "db-sea-orm")]
impl From<sea_orm_migration::MigrationStatus> for MigrationStatus {
    fn from(value: sea_orm_migration::MigrationStatus) -> Self {
        match value {
            sea_orm_migration::MigrationStatus::Applied => Self::Applied,
            sea_orm_migration::MigrationStatus::Pending => Self::Pending,
        }
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Migrator<S>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    /// Apply pending migrations. Returns the number of migrations that were successfully
    /// applied.
    async fn up(&self, state: &S, args: &UpArgs) -> RoadsterResult<usize>;

    /// Roll back previous applied migrations. Returns the number of migrations that were
    /// successfully rolled back.
    async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize>;

    /// Get the status of all migrations in this [`Migrator`].
    async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>>;
}

#[cfg(feature = "db-sea-orm")]
pub struct SeaOrmMigrator<M>
where
    M: sea_orm_migration::MigratorTrait + Send + Sync,
{
    migrator: M,
}

#[cfg(feature = "db-sea-orm")]
impl<M> SeaOrmMigrator<M>
where
    M: sea_orm_migration::MigratorTrait + Send + Sync,
{
    pub fn new(migrator: M) -> Self {
        Self { migrator }
    }
}

#[cfg(feature = "db-diesel")]
pub struct DieselMigrator<C>
where
    C: Send + Connection + diesel_migrations::MigrationHarness<C::Backend>,
{
    migrator: Box<dyn diesel::migration::MigrationSource<C::Backend> + Send + Sync>,
    // Diesel connections don't implement `Sync`, so we need to wrap the `PhantomData` in a
    // `Mutex` to satisfy `Sync` trait bounds elsewhere.
    // https://github.com/diesel-rs/diesel/issues/190
    _conn: PhantomData<Mutex<C>>,
}

#[cfg(feature = "db-diesel")]
impl<C> DieselMigrator<C>
where
    C: Connection + Send + diesel_migrations::MigrationHarness<C::Backend>,
{
    pub fn new(
        migrator: impl 'static + diesel::migration::MigrationSource<C::Backend> + Send + Sync,
    ) -> Self {
        Self {
            migrator: Box::new(migrator),
            _conn: Default::default(),
        }
    }
}

// todo: Maybe instead of implementing for any `T: sea_orm_migration::MigratorTrait`, create
//  wrapper structs (e.g. `SeaOrmMigrator<T: sea_orm_migration::MigratorTrait>(T)`
//  and `DieselMigrator<T: MigrationHarness>(T)`) and implement `Migrator` for the wrapper structs.
//
// todo: conflicting def for `diesel_migrations::EmbeddedMigrations` because of using
//  `sea_orm_migration::MigratorTrait` trait instead of a concrete type. For now, use `cfg` flags
//  to disable the default impl diesel is enabled. This is not ideal because enabling
//  diesel when sea-orm is also disabled causes the impl to be removed which is a semver
//  breaking change. However, most consumers should not have both diesel and sea-orm enabled
//  at the same time, so we'll accept this trade-off for now. I think there's a Rust feature in
//  nightly that would improve this that we could use in the future.
#[cfg(all(not(feature = "db-diesel"), feature = "db-sea-orm"))]
#[async_trait::async_trait]
impl<T, S> crate::migration::Migrator<S> for T
where
    T: sea_orm_migration::MigratorTrait + Send + Sync,
    S: Clone + Send + Sync + 'static,
    crate::app::context::AppContext: axum_core::extract::FromRef<S>,
{
    #[tracing::instrument(skip_all)]
    async fn up(&self, state: &S, args: &UpArgs) -> crate::error::RoadsterResult<usize> {
        use axum_core::extract::FromRef;
        use std::cmp::min;

        let context = crate::app::context::AppContext::from_ref(state);
        let pending = T::get_pending_migrations(context.db()).await?;

        let to_run = if let Some(steps) = args.steps {
            min(steps, pending.len())
        } else {
            pending.len()
        };

        T::up(context.db(), args.steps.map(|steps| steps as u32)).await?;

        // Assume all of the pending steps (up to `args.steps` count) ran successfully.
        Ok(to_run)
    }

    #[tracing::instrument(skip_all)]
    async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize> {
        use axum_core::extract::FromRef;
        use std::cmp::min;

        let context = crate::app::context::AppContext::from_ref(state);
        let applied = T::get_applied_migrations(context.db()).await?;

        let to_roll_back = if let Some(steps) = args.steps {
            min(steps, applied.len())
        } else {
            applied.len()
        };

        T::down(context.db(), args.steps.map(|steps| steps as u32)).await?;

        // Assume all of the applied steps (up to `args.steps` count) were rolled back successfully.
        Ok(to_roll_back)
    }

    #[tracing::instrument(skip_all)]
    async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>> {
        use axum_core::extract::FromRef;

        let context = crate::app::context::AppContext::from_ref(state);

        let migrations = T::get_migration_with_status(context.db())
            .await?
            .into_iter()
            .map(|migration| migration.into())
            .collect();

        Ok(migrations)
    }
}

// #[cfg_attr(feature = "db-diesel", derive(diesel::MultiConnection))]
// pub enum AnyConnection {
//     Postgresql(diesel::PgConnection),
//     // Mysql(diesel::MysqlConnection),
//     // Sqlite(diesel::SqliteConnection),
// }

#[cfg(feature = "db-sea-orm")]
#[async_trait]
impl<S, M> Migrator<S> for SeaOrmMigrator<M>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    M: sea_orm_migration::MigratorTrait + Send + Sync,
{
    async fn up(&self, state: &S, args: &UpArgs) -> RoadsterResult<usize> {
        todo!()
    }

    async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize> {
        todo!()
    }

    async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>> {
        todo!()
    }
}

#[cfg(feature = "db-diesel")]
#[async_trait::async_trait]
impl<S, C> Migrator<S> for DieselMigrator<C>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    C: Connection + Send + diesel_migrations::MigrationHarness<C::Backend>,
{
    #[tracing::instrument(skip_all)]
    async fn up(&self, state: &S, args: &UpArgs) -> RoadsterResult<usize> {
        use std::cmp::min;

        tracing::info!("Started applying migrations");

        // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
        //  aren't satisfied by the pooled connections currently, at least not for async
        //  connection pools.
        let context = AppContext::from_ref(state);

        let mut conn: C = Connection::establish(context.config().database.uri.as_ref())?;
        let pending =
            conn.pending_migrations(DieselMigrationSourceWrapper::try_from(&self.migrator)?)?;
        //
        let pending = if let Some(steps) = args.steps {
            let steps = min(steps, pending.len());
            pending.into_iter().take(steps).collect()
        } else {
            pending
        };

        let completed = conn.run_migrations(&pending)?;
        let completed = completed.len();

        tracing::info!("Completed applying {completed} migrations");

        Ok(completed)
    }

    #[tracing::instrument(skip_all)]
    async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize> {
        use diesel_migrations::MigrationError;
        use itertools::Itertools;
        use std::cmp::min;
        use std::collections::HashMap;

        tracing::info!("Started rolling back migrations");

        // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
        //  aren't satisfied by the pooled connections currently, at least not for async
        //  connection pools.
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
        // Todo: which order are applied migrations returned in?
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
            conn.revert_migration(&migration_to_revert)?;
        }

        tracing::info!("Completed rolling back {to_roll_back} migrations");

        Ok(to_roll_back)
    }

    #[tracing::instrument(skip_all)]
    async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>> {
        use std::collections::HashMap;

        // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
        //  aren't satisfied by the pooled connections currently, at least not for async
        //  connection pools.
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
            .map(|m: Box<dyn diesel::migration::Migration<C::Backend>>| {
                (m.name().version().as_owned(), m)
            })
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
//
// // todo: implement for file based migrations too
// // todo: support other db backends
// #[cfg(feature = "db-diesel")]
// #[async_trait]
// impl<S> Migrator<S> for diesel_migrations::EmbeddedMigrations
// where
//     S: Clone + Send + Sync + 'static,
//     AppContext: FromRef<S>,
// {
//     #[tracing::instrument(skip_all)]
//     async fn up(&self, state: &S, args: &UpArgs) -> RoadsterResult<usize> {
//         use diesel::Connection;
//         use diesel_migrations::MigrationHarness;
//         use std::cmp::min;
//
//         tracing::info!("Started applying migrations");
//
//         // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
//         //  aren't satisfied by the pooled connections currently, at least not for async
//         //  connection pools.
//         let context = AppContext::from_ref(state);
//         let mut conn = diesel::PgConnection::establish(context.config().database.uri.as_ref())?;
//
//         let pending = conn.pending_migrations(DieselMigrationSourceWrapper::try_from(self)?)?;
//         tracing::debug!("pending: {}", pending.len());
//
//         let pending = if let Some(steps) = args.steps {
//             let steps = min(steps, pending.len());
//             pending.into_iter().take(steps).collect()
//         } else {
//             pending
//         };
//
//         let completed = conn.run_migrations(&pending)?;
//         let completed = completed.len();
//
//         tracing::info!("Completed applying {completed} migrations");
//
//         Ok(completed)
//     }
//
//     #[tracing::instrument(skip_all)]
//     async fn down(&self, state: &S, args: &DownArgs) -> RoadsterResult<usize> {
//         use diesel::migration::MigrationSource;
//         use diesel::Connection;
//         use diesel_migrations::MigrationError;
//         use diesel_migrations::MigrationHarness;
//         use itertools::Itertools;
//         use std::cmp::min;
//         use std::collections::HashMap;
//
//         tracing::info!("Started rolling back migrations");
//
//         // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
//         //  aren't satisfied by the pooled connections currently, at least not for async
//         //  connection pools.
//         let context = AppContext::from_ref(state);
//         let mut conn = diesel::PgConnection::establish(context.config().database.uri.as_ref())?;
//
//         let to_roll_back = conn.applied_migrations()?.len();
//         let to_roll_back = if let Some(steps) = args.steps {
//             min(steps, to_roll_back)
//         } else {
//             to_roll_back
//         };
//
//         // This is mostly copied from the default `MigrationHarness#revert_all_migrations`
//         // implementation, with a slight modification to only revert the first `to_roll_back`
//         // migrations.
//         // Todo: which order are applied migrations returned in?
//         let applied_versions = conn
//             .applied_migrations()?
//             .into_iter()
//             .take(to_roll_back)
//             .collect_vec();
//         let mut migrations: HashMap<_, _> = self
//             .migrations()?
//             .into_iter()
//             .map(|m| (m.name().version().as_owned(), m))
//             .collect();
//
//         for version in applied_versions {
//             let migration_to_revert = migrations
//                 .remove(&version)
//                 .ok_or(MigrationError::UnknownMigrationVersion(version))?;
//             conn.revert_migration(&migration_to_revert)?;
//         }
//
//         tracing::info!("Completed rolling back {to_roll_back} migrations");
//
//         Ok(to_roll_back)
//     }
//
//     #[tracing::instrument(skip_all)]
//     async fn status(&self, state: &S) -> RoadsterResult<Vec<MigrationInfo>> {
//         use diesel::migration::MigrationSource;
//         use diesel::Connection;
//         use diesel_migrations::MigrationHarness;
//         use std::collections::HashMap;
//
//         // todo: Is there a way to use a pooled connection instead? It seems like the trait bounds
//         //  aren't satisfied by the pooled connections currently, at least not for async
//         //  connection pools.
//         let context = AppContext::from_ref(state);
//         let mut conn = diesel::PgConnection::establish(context.config().database.uri.as_ref())?;
//
//         let pending = conn
//             .pending_migrations(DieselMigrationSourceWrapper::try_from(self)?)?
//             .into_iter()
//             .map(|migration| {
//                 MigrationInfo::builder()
//                     .name(migration.name().to_string())
//                     .status(MigrationStatus::Pending)
//                     .build()
//             });
//
//         let migrations: HashMap<_, _> = self
//             .migrations()?
//             .into_iter()
//             .map(|m: Box<dyn diesel::migration::Migration<diesel::pg::Pg>>| {
//                 (m.name().version().as_owned(), m)
//             })
//             .collect();
//
//         let applied = conn
//             .applied_migrations()?
//             .into_iter()
//             .map(|version| {
//                 let name = migrations
//                     .get(&version)
//                     .map(|migration| migration.name().to_string())
//                     .unwrap_or(version.to_string());
//                 MigrationInfo::builder()
//                     .name(name)
//                     .status(MigrationStatus::Applied)
//                     .build()
//             })
//             .rev();
//
//         let migrations = applied.into_iter().chain(pending.into_iter()).collect();
//
//         Ok(migrations)
//     }
// }

/// [`MigrationHarness#run_pending_migrations`] takes an owned version of the
/// [`diesel_migrations::MigrationSource`], but our [`Migrator`] trait uses a reference. Because
/// [`diesel_migrations::EmbeddedMigrations`] doesn't implement [`Clone`], we can't directly use
/// it in our [`Migrator`]. However, [`diesel::migration::MigrationSource#migrations`] does take a
/// reference, so we can wrap it, pre-fetch the list of migrations, and then return them from the
/// wrapper's impl.
#[cfg(feature = "db-diesel")]
struct DieselMigrationSourceWrapper<DB: diesel::backend::Backend> {
    migrations: std::sync::Mutex<Option<Vec<Box<dyn diesel::migration::Migration<DB>>>>>,
}

#[cfg(feature = "db-diesel")]
impl<DB> TryFrom<&Box<dyn diesel::migration::MigrationSource<DB> + Send + Sync>>
    for DieselMigrationSourceWrapper<DB>
where
    DB: diesel::backend::Backend,
{
    type Error = Box<dyn std::error::Error + Send + Sync>;
    fn try_from(
        value: &Box<dyn diesel::migration::MigrationSource<DB> + Send + Sync>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            migrations: std::sync::Mutex::new(Some(value.migrations()?)),
        })
    }
}

#[cfg(feature = "db-diesel")]
impl<DB: diesel::backend::Backend> diesel::migration::MigrationSource<DB>
    for DieselMigrationSourceWrapper<DB>
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
            None => Err(anyhow::anyhow!("EmbeddedMigrationsWrapper#migrations was called twice! This is not supported as the migrations were removed by the first call.").into()),
        }
    }
}
