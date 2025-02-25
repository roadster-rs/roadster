//! Migration to create a SQL trigger to automatically update the [Timestamps::UpdatedAt] column of
//! a row in the `user` table whenever the row is updated.
//!
//! Expects to be run after [crate::db::migration::sea_orm::timestamp::m20240723_201404_add_update_timestamp_function::Migration],
//! or another equivalent [Migration].
//!
//! Note: Currently only supports Postgres. If another DB is used, will do nothing.

use crate::db::migration::sea_orm::timestamp::Timestamps;
use crate::db::migration::sea_orm::timestamp::{
    exec_create_update_timestamp_trigger, exec_drop_update_timestamp_trigger,
};
use crate::db::migration::sea_orm::user::User;
use sea_orm_migration::prelude::*;

const TABLE: User = User::Table;
const COLUMN: Timestamps = Timestamps::UpdatedAt;

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
