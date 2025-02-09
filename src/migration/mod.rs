//! This module provides pre-built SeaORM migrations for table schemas that are applicable
//! across many different applications and problem spaces.
//!
//! Additionally, some utilities are provided to create some common column types.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use diesel_migrations::MigrationHarness;
use futures::StreamExt;
use std::error::Error;
use std::future::Future;
use std::sync::Mutex;
use tokio::io::AsyncReadExt;

#[cfg(feature = "db-sea-orm")]
pub mod sea_orm;

pub type BoxedMigrator<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
= Box<dyn Migrator<S> + Send + Sync>;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Migrator<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    async fn up(&self, state: &S) -> RoadsterResult<()>;
}

// #[cfg(feature = "db-sea-orm")]
// #[async_trait]
// impl<T> Migrator for T
// where
//     T: Send + Sync + sea_orm_migration::MigratorTrait,
// {
//     async fn up<S>(&self, state: &S) -> RoadsterResult<()>
//     where
//         S: Clone + Send + Sync + 'static,
//         AppContext: FromRef<S>,
//     {
//         let context = AppContext::from_ref(state);
//         T::up(context.db(), None).await?;
//         Ok(())
//     }
// }
// const m: EmbeddedMigrations = embed_migrations!("");
//
// struct FooConnectionManager;
// impl bb8::ManageConnection for FooConnectionManager {
//     type Connection = ();
//     type Error = ();
//
//     fn connect(&self) -> impl Future<Output = Result<Self::Connection, Self::Error>> + Send {
//         todo!()
//     }
//
//     fn is_valid(
//         &self,
//         conn: &mut Self::Connection,
//     ) -> impl Future<Output = Result<(), Self::Error>> + Send {
//         todo!()
//     }
//
//     fn has_broken(&self, conn: &mut Self::Connection) -> bool {
//         todo!()
//     }
// }

struct EmbeddedMigrationsWrapper<DB: diesel::backend::Backend> {
    migrations: Mutex<Option<Vec<Box<dyn diesel::migration::Migration<DB>>>>>,
}

impl<DB: diesel::backend::Backend> TryFrom<&diesel_migrations::EmbeddedMigrations>
    for EmbeddedMigrationsWrapper<DB>
{
    type Error = Box<dyn Error + Send + Sync>;
    fn try_from(value: &diesel_migrations::EmbeddedMigrations) -> Result<Self, Self::Error> {
        Ok(Self {
            migrations: Mutex::new(Some(
                <diesel_migrations::EmbeddedMigrations as diesel::migration::MigrationSource<
                    DB,
                >>::migrations(value)?,
            )),
        })
    }
}

impl<DB: diesel::backend::Backend> diesel::migration::MigrationSource<DB>
    for EmbeddedMigrationsWrapper<DB>
{
    fn migrations(
        &self,
    ) -> diesel::migration::Result<Vec<Box<dyn diesel::migration::Migration<DB>>>> {
        let mut migrations = self
            .migrations
            .lock()
            // todo: poison error enum variant
            .map_err(|err| crate::error::Error::from(anyhow!("{err}")))?;

        match migrations.take() {
            Some(migrations) => Ok(migrations),
            None => Ok(vec![]),
        }
    }
}

#[cfg(feature = "db-diesel")]
#[async_trait]
impl<S> Migrator<S> for diesel_migrations::EmbeddedMigrations
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    async fn up(&self, state: &S) -> RoadsterResult<()> {
        let context = AppContext::from_ref(state);
        // let mut conn = context.diesel().get().await?;
        // todo: how to use a pooled connection instead of a dedicated connection?
        let mut conn = context.diesel().dedicated_connection().await?;
        // todo: how to get this to work without an async connection wrapper?
        // todo: other db backend types?
        let mut conn = diesel_async::async_connection_wrapper::AsyncConnectionWrapper::<
            diesel_async::AsyncPgConnection,
        >::from(conn);

        let migration_wrapper = EmbeddedMigrationsWrapper::try_from(self)?;

        conn.run_pending_migrations(migration_wrapper)?;
        Ok(())
    }
}
