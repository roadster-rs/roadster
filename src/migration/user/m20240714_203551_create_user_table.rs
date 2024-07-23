use crate::migration::check::str_not_empty;
use crate::migration::schema::{pk_uuid, table};
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{string, string_null, string_uniq, timestamp_with_time_zone_null};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(create_table()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(drop_table()).await
    }
}

#[derive(DeriveIden)]
enum User {
    Table,
    Id,
    Name,
    Username,
    Email,
    Password,
    EmailConfirmationSentAt,
    EmailConfirmationToken,
    EmailConfirmedAt,
    LastSignInAt,
    RecoverySentAt,
    RecoveryToken,
    EmailChangeSentAt,
    /// Token sent to the new email to confirm it's a valid email and the user has access to it.
    EmailChangeTokenNew,
    /// Token sent to the current email to confirm the user authorized the email change.
    EmailChangeTokenCurrent,
    /// When the user was deleted.
    DeletedAt,
}

fn create_table() -> TableCreateStatement {
    table(User::Table)
        .col(pk_uuid(User::Id))
        .col(string(User::Name).check(str_not_empty(User::Name)))
        .col(string_uniq(User::Username).check(str_not_empty(User::Username)))
        .col(string_uniq(User::Email).check(str_not_empty(User::Email)))
        .col(string(User::Password))
        .col(timestamp_with_time_zone_null(User::EmailConfirmationSentAt))
        .col(string_null(User::EmailConfirmationToken))
        .col(timestamp_with_time_zone_null(User::EmailConfirmedAt))
        .col(timestamp_with_time_zone_null(User::LastSignInAt))
        .col(timestamp_with_time_zone_null(User::RecoverySentAt))
        .col(string_null(User::RecoveryToken))
        .col(timestamp_with_time_zone_null(User::EmailChangeSentAt))
        .col(string_null(User::EmailChangeTokenNew))
        .col(string_null(User::EmailChangeTokenCurrent))
        .col(timestamp_with_time_zone_null(User::DeletedAt))
        .to_owned()
}

fn drop_table() -> TableDropStatement {
    Table::drop().table(User::Table).to_owned()
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_table() {
        let query = super::create_table();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_table() {
        let query = super::drop_table();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }
}
