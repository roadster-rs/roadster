//! Migration to create a SQL function to update the [`Timestamps::UpdatedAt`] column for a row
//! with the current timestamp.
//!
//! Note: Currently only supports Postgres. If another DB is used, will do nothing.

use crate::migration::timestamp::Timestamps;
use crate::migration::timestamp::{
    exec_create_update_timestamp_function, exec_drop_update_timestamp_function,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const COLUMN: Timestamps = Timestamps::UpdatedAt;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_create_update_timestamp_function(manager, COLUMN).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_drop_update_timestamp_function(manager, COLUMN).await
    }
}
