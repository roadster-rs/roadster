use crate::migration::collation::Collation;
use crate::migration::user::User;
use sea_orm::{DbBackend, Statement, TransactionTrait};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let transaction = db.begin().await?;

        for statement in up_statements(manager.get_database_backend()) {
            transaction.execute(statement).await?;
        }

        transaction.commit().await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let transaction = db.begin().await?;

        for statement in down_statements(manager.get_database_backend()) {
            transaction.execute(statement).await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}

fn up_statements(db_backend: DbBackend) -> Vec<Statement> {
    vec![
        alter_table_set_case_insensitive_collation(db_backend, User::Username),
        alter_table_set_case_insensitive_collation(db_backend, User::Email),
    ]
}

fn alter_table_set_case_insensitive_collation(db_backend: DbBackend, column: User) -> Statement {
    Statement::from_string(
        db_backend,
        format!(
            r#"ALTER table "{}" ALTER COLUMN "{}" type {} COLLATE {}"#,
            User::Table.to_string(),
            column.to_string(),
            "varchar",
            Collation::CaseInsensitive.to_string(),
        ),
    )
}

fn down_statements(db_backend: DbBackend) -> Vec<Statement> {
    vec![
        alter_table_reset_collation(db_backend, User::Username),
        alter_table_reset_collation(db_backend, User::Email),
    ]
}

fn alter_table_reset_collation(db_backend: DbBackend, column: User) -> Statement {
    Statement::from_string(
        db_backend,
        format!(
            r#"ALTER table "{}" ALTER COLUMN "{}" type {} COLLATE "{}""#,
            User::Table.to_string(),
            column.to_string(),
            "varchar",
            Collation::Default.to_string(),
        ),
    )
}

#[cfg(test)]
mod tests {
    use crate::testing::snapshot::TestCase;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use sea_orm::DbBackend;

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn up_statements(_case: TestCase, #[case] db_backend: DbBackend) {
        let query = super::up_statements(db_backend);

        assert_debug_snapshot!(query);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn down_statements(_case: TestCase, #[case] db_backend: DbBackend) {
        let query = super::down_statements(db_backend);

        assert_debug_snapshot!(query);
    }
}
