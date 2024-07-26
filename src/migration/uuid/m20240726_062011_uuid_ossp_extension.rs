//! Migration to create the `uuid-ossp` Postgres extension to enable using uuid generation
//! functions in a Postgres database.
//!
//! See: <https://www.postgresql.org/docs/current/uuid-ossp.html>

use crate::migration::uuid::{create_uuid_ossp_extension, drop_uuid_ossp_extension};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(create_uuid_ossp_extension())
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(drop_uuid_ossp_extension())
            .await?;

        Ok(())
    }
}
