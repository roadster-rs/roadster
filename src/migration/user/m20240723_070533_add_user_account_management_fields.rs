//! Migrations to add some common account management fields to the `user` table. The following
//! fields are added:
//!
//! - EmailConfirmationSentAt
//! - EmailConfirmationToken
//! - EmailConfirmedAt
//! - LastSignInAt
//! - RecoverySentAt
//! - RecoveryToken
//! - EmailChangeSentAt
//! - EmailChangeTokenNew
//! - EmailChangeTokenCurrent
//! - DeletedAt

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
        .add_column_if_not_exists(timestamp_with_time_zone_null(User::EmailConfirmationSentAt))
        .add_column_if_not_exists(string_null(User::EmailConfirmationToken))
        .add_column_if_not_exists(timestamp_with_time_zone_null(User::EmailConfirmedAt))
        .add_column_if_not_exists(timestamp_with_time_zone_null(User::LastSignInAt))
        .add_column_if_not_exists(timestamp_with_time_zone_null(User::RecoverySentAt))
        .add_column_if_not_exists(string_null(User::RecoveryToken))
        .add_column_if_not_exists(timestamp_with_time_zone_null(User::EmailChangeSentAt))
        .add_column_if_not_exists(string_null(User::EmailChangeTokenNew))
        .add_column_if_not_exists(string_null(User::EmailChangeTokenCurrent))
        .add_column_if_not_exists(timestamp_with_time_zone_null(User::DeletedAt))
        .to_owned()
}

fn alter_table_drop_columns() -> TableAlterStatement {
    Table::alter()
        .table(User::Table)
        .drop_column(User::EmailConfirmationSentAt)
        .drop_column(User::EmailConfirmationToken)
        .drop_column(User::EmailConfirmedAt)
        .drop_column(User::LastSignInAt)
        .drop_column(User::RecoverySentAt)
        .drop_column(User::RecoveryToken)
        .drop_column(User::EmailChangeSentAt)
        .drop_column(User::EmailChangeTokenNew)
        .drop_column(User::EmailChangeTokenCurrent)
        .drop_column(User::DeletedAt)
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
