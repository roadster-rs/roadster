//! Migration to add a [User::PasswordUpdatedAt] column to the `user` table.

use crate::migration::sea_orm::user::User;
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::timestamp_with_time_zone;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(add_column()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(drop_column()).await
    }
}

fn add_column() -> TableAlterStatement {
    Table::alter()
        .table(User::Table)
        .add_column(
            timestamp_with_time_zone(User::PasswordUpdatedAt).default(Expr::current_timestamp()),
        )
        .to_owned()
}

fn drop_column() -> TableAlterStatement {
    Table::alter()
        .table(User::Table)
        .drop_column(User::PasswordUpdatedAt)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use sea_orm_migration::prelude::PostgresQueryBuilder;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn add_column() {
        let stmt = super::add_column();

        assert_snapshot!(stmt.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_column() {
        let stmt = super::drop_column();

        assert_snapshot!(stmt.to_string(PostgresQueryBuilder));
    }
}
