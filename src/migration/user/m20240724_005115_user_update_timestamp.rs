//! Migration to create a SQL trigger to automatically update the [Timestamps::UpdatedAt] column of
//! a row in the `user` table whenever the row is updated.
//!
//! Expects to be run after [crate::migration::timestamp::m20240723_201404_add_update_timestamp_function::Migration],
//! or another equivalent [Migration].
//!
//! Note: Currently only supports Postgres. If another DB is used, will do nothing.

use crate::migration::schema::Timestamps;
use crate::migration::timestamp::{
    exec_create_update_timestamp_trigger, exec_drop_update_timestamp_trigger,
};
use crate::migration::user::User;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const TABLE: User = User::Table;
const COLUMN: Timestamps = Timestamps::UpdatedAt;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_create_update_timestamp_trigger(manager, TABLE, COLUMN).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_drop_update_timestamp_trigger(manager, TABLE, COLUMN).await
    }
}
