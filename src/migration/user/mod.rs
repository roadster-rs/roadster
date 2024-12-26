use crate::migration::collation::m20241022_065427_case_insensitive_collation;
use crate::migration::timestamp::m20240723_201404_add_update_timestamp_function;
use sea_orm_migration::prelude::*;

mod create_and_drop_table;
pub mod m20240714_203550_create_user_table_int_pk;
pub mod m20240714_203551_create_user_table_uuid_pk;
pub mod m20240723_070533_add_user_account_management_fields;
pub mod m20240724_005115_user_update_timestamp;
pub mod m20240729_000812_password_updated_at;
pub mod m20240729_002549_password_updated_at_function;
pub mod m20240729_002615_password_updated_at_trigger;
pub mod m20241022_072216_case_insensitive_username_email;
pub mod m20241226_080735_pending_email;
#[cfg(test)]
mod tests;

/// Contains the identifiers/fields created by all the `user` migrations.
#[derive(DeriveIden)]
#[non_exhaustive]
pub enum User {
    Table,
    Id,
    Name,
    Username,
    Email,
    Password,
    /// When the user's password was updated. Defaults to the [`UpdatedAt`][crate::migration::timestamp::Timestamps]
    /// time. Useful in the event users' passwords may have been compromised and the application
    /// needs to enforce that users update their passwords.
    ///
    /// Updated automatically when the [User::Password] is updated, assuming the following
    /// migrations are applied:
    /// 1. [m20240729_002549_password_updated_at_function::Migration]
    /// 2. [m20240729_002615_password_updated_at_trigger::Migration]
    PasswordUpdatedAt,
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
    /// The user's new email address that has not yet been confirmed.
    PendingEmail,
}

/// The collection of migrations defined to create a `user` table. Relevant [MigrationTrait]s
/// authored by `roadster` will be added here.
///
/// Note that the migration uses a `UUID` field for the `id` Primary Key field. If you would like
/// to use a `BIGINT` instead, you can do one of the following:
///
/// 1. Use the [m20240714_203550_create_user_table_int_pk::Migration] instead -- simply add it to
///    your main [MigratorTrait] implementation before the migrations from [UserMigrator].
/// 2. Add an `alter table` migration after the migrations from [UserMigrator].
#[non_exhaustive]
pub struct UserMigrator;

#[async_trait::async_trait]
impl MigratorTrait for UserMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240714_203551_create_user_table_uuid_pk::Migration),
            Box::new(m20240723_070533_add_user_account_management_fields::Migration),
            Box::new(m20240723_201404_add_update_timestamp_function::Migration),
            Box::new(m20240724_005115_user_update_timestamp::Migration),
            Box::new(m20240729_000812_password_updated_at::Migration),
            Box::new(m20240729_002549_password_updated_at_function::Migration),
            Box::new(m20240729_002615_password_updated_at_trigger::Migration),
            Box::new(m20241022_065427_case_insensitive_collation::Migration),
            Box::new(m20241022_072216_case_insensitive_username_email::Migration),
            Box::new(m20241226_080735_pending_email::Migration),
        ]
    }
}
