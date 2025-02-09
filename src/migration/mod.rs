//! This module provides pre-built SeaORM migrations for table schemas that are applicable
//! across many different applications and problem spaces.
//!
//! Additionally, some utilities are provided to create some common column types.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

#[cfg(feature = "db-sea-orm")]
pub mod sea_orm;

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Migrator {
    async fn up<S>(state: &S) -> RoadsterResult<()>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>;
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

// #[cfg(feature = "db-diesel")]
// #[async_trait]
// impl Migrator for EmbeddedMigrations {
//     async fn up<S>(state: &S) -> RoadsterResult<()>
//     where
//         S: Clone + Send + Sync + 'static,
//         AppContext: FromRef<S>,
//     {
//         let context = AppContext::from_ref(state);
//         let conn = context.diesel().get().await?;
//         conn.run_pending_migrations(m)?;
//         // conn.run_pending_migrations(self);
//         Ok(())
//     }
// }
