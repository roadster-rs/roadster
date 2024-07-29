use crate::migration::timestamp::{
    exec_create_update_timestamp_trigger, exec_drop_update_timestamp_trigger,
};
use crate::migration::user::User;
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
