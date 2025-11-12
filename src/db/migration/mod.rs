//! This module provides pre-built SeaORM migrations for table schemas that are applicable
//! across many different applications and problem spaces.
//!
//! Additionally, some utilities are provided to create some common column types.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde_derive::Serialize;
use strum_macros::{EnumString, IntoStaticStr};

#[cfg(feature = "db-diesel")]
pub mod diesel;
#[cfg(feature = "db-sea-orm")]
pub mod sea_orm;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, bon::Builder)]
#[cfg_attr(feature = "cli", derive(clap::Parser))]
#[non_exhaustive]
pub struct UpArgs {
    /// The number of pending migration steps to apply.
    #[cfg_attr(feature = "cli", clap(short = 'n', long))]
    pub steps: Option<usize>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, bon::Builder)]
#[cfg_attr(feature = "cli", derive(clap::Parser))]
#[non_exhaustive]
pub struct DownArgs {
    /// The number of applied migration steps to roll back.
    #[cfg_attr(feature = "cli", clap(short = 'n', long))]
    pub steps: Option<usize>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, bon::Builder)]
pub struct MigrationInfo {
    pub name: String,
    pub status: MigrationStatus,
}

#[derive(Debug, Serialize, EnumString, IntoStaticStr)]
pub enum MigrationStatus {
    Applied,
    Pending,
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Migrator<S>: Send + Sync
where
    S: 'static + Send + Sync + Clone,
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
