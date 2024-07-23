//! Migrations to create a basic `user` table that contains the following fields:
//!
//! - Id (UUID)
//! - Name
//! - Username
//! - Email
//! - Password
//!
//! To add more fields, use the other migrations in the `user` mod.

use crate::migration::user::create_table::{create_table_uuid_pk, drop_table};
use sea_orm_migration::prelude::*;

#[derive(Default, DeriveMigrationName)]
#[non_exhaustive]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_table_uuid_pk()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(drop_table()).await
    }
}
