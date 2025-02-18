//! Migration to create a SQL trigger to automatically update the [User::PasswordUpdatedAt] column
//! of a row in the `user` table whenever the row's [User::Password] column is updated.
//!
//! Expects to be run after [crate::migration::sea_orm::user::m20240729_002549_password_updated_at_function::Migration].
//!
//! Note: Currently only supports Postgres. If another DB is used, will do nothing.

use crate::migration::sea_orm::timestamp::{
    exec_create_update_timestamp_trigger, exec_drop_update_timestamp_trigger,
};
use crate::migration::sea_orm::user::User;
use sea_orm_migration::prelude::*;

const TABLE: User = User::Table;
const COLUMN: User = User::PasswordUpdatedAt;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_create_update_timestamp_trigger(manager, TABLE, COLUMN).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_drop_update_timestamp_trigger(manager, TABLE, COLUMN).await
    }
}
