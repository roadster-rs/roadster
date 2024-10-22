//! Migration to create a case-insensitive collation.
//!
//! See: <https://www.postgresql.org/docs/current/collation.html#COLLATION-NONDETERMINISTIC>
//!
//! Note: Currently only supports Postgres. If another DB is used, will do nothing.

use crate::migration::collation::{
    exec_create_case_insensitive_collation, exec_drop_case_insensitive_collation,
};
use async_trait::async_trait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_create_case_insensitive_collation(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        exec_drop_case_insensitive_collation(manager).await
    }
}
