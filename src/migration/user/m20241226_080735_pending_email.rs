use crate::migration::user::User;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(alter_table_add_columns()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.alter_table(alter_table_drop_columns()).await
    }
}

fn alter_table_add_columns() -> TableAlterStatement {
    Table::alter()
        .table(User::Table)
        .add_column_if_not_exists(string_null(User::PendingEmail))
        .to_owned()
}

fn alter_table_drop_columns() -> TableAlterStatement {
    Table::alter()
        .table(User::Table)
        .drop_column(User::PendingEmail)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn alter_table_add_columns() {
        let query = super::alter_table_add_columns();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn alter_table_drop_columns() {
        let query = super::alter_table_drop_columns();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }
}
