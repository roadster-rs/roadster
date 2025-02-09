//! Migration to create a SQL function to update the [User::PasswordUpdatedAt] column for a row
//! with the current timestamp, but only if the [User::Password] column was updated.
//!
//! Note: Currently only supports Postgres. If another DB is used, will do nothing.

use crate::migration::sea_orm::timestamp::{
    exec_create_update_timestamp_function_dep_column, exec_drop_update_timestamp_function,
};
use crate::migration::sea_orm::user::User;
use sea_orm_migration::prelude::*;

const TIMESTAMP_COLUMN: User = User::PasswordUpdatedAt;
const DEPENDENT_COLUMN: User = User::Password;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_create_update_timestamp_function_dep_column(
            manager,
            TIMESTAMP_COLUMN,
            DEPENDENT_COLUMN,
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_drop_update_timestamp_function(manager, TIMESTAMP_COLUMN).await
    }
}
