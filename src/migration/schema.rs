//! Utility methods to create common column types in table create/alter statements.
//!
//! These utilities are similar to the ones provided by [SeaORM][sea_orm_migration::schema] and
//! [Loco](https://github.com/loco-rs/loco/blob/be7ead6e2503731aea252ed8dc6542d74f2c2e4f/src/schema.rs),
//! but with some minor differences. For example, our updated/created at timestamps include the
//! timezone, while SeaORM/Loco do not.

use sea_orm_migration::{prelude::*, schema::*};

/// Timestamp related fields.
#[derive(DeriveIden)]
#[non_exhaustive]
pub enum Timestamps {
    /// When the row was created. When used with the [timestamps] method, will default to
    /// the current timestamp (with timezone).
    CreatedAt,
    /// When the row was updated. When used with the [timestamps] method, will be initially set to
    /// the current timestamp (with timezone).
    ///
    /// To automatically update the value for a row whenever the row is updated, include the
    /// [crate::migration::timestamp::m20240723_201404_add_update_timestamp_function::Migration]
    /// in your [MigratorTrait] implementation, along with a [MigrationTrait] for your table
    /// that add a trigger to update the column. Helper methods are provided for this in
    /// the [crate::migration::timestamp] module. Specifically, see:
    /// - [crate::migration::timestamp::exec_create_update_timestamp_trigger]
    /// - [crate::migration::timestamp::exec_drop_update_timestamp_trigger]
    ///
    /// Note that the auto-updates mentioned above are currently only supported on Postgres. If
    /// an app is using a different DB, it will need to manually update the timestamp when updating
    /// a row.
    UpdatedAt,
}

/// Create a table if it does not exist yet and add some default columns
/// (e.g., create/update timestamps).
pub fn table<T: IntoTableRef>(name: T) -> TableCreateStatement {
    timestamps(Table::create().table(name).if_not_exists().to_owned())
}

/// Add "timestamp with time zone" columns (`CreatedAt` and `UpdatedAt`) to a table.
/// The default for each column is the current timestamp.
pub fn timestamps(mut table: TableCreateStatement) -> TableCreateStatement {
    table
        .col(timestamp_with_time_zone(Timestamps::CreatedAt).default(Expr::current_timestamp()))
        .col(timestamp_with_time_zone(Timestamps::UpdatedAt).default(Expr::current_timestamp()))
        .to_owned()
}

/// Create an auto-incrementing primary key column using [BigInteger][sea_orm::sea_query::ColumnType::BigInteger]
/// as the column type.
pub fn pk_bigint_auto<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    big_integer(name).primary_key().auto_increment().to_owned()
}

/// Create a primary key column using [Uuid][sea_orm::sea_query::ColumnType::Uuid] as the column
/// type. No default value is provided, so it needs to be generated/provided by the application.
pub fn pk_uuid<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name).uuid().primary_key().to_owned()
}

/// Create a primary key column using [Uuid][sea_orm::sea_query::ColumnType::Uuid] as the column
/// type. A new v4 UUID will be generated as the default if no value is provided by the application.
///
/// Note: This requires that your database supports generating v4 UUIDs using a method named
/// `uuid_generate_v4()`.
pub fn pk_uuidv4<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    pk_uuid_default(name, Expr::cust("uuid_generate_v4()"))
}

/// Create a primary key column using [Uuid][sea_orm::sea_query::ColumnType::Uuid] as the column
/// type. A new v7 UUID will be generated as the default if no value is provided by the application.
///
/// Note: This requires that your database supports generating v7 UUIDs using a method named
/// `uuid_generate_v7()`.
pub fn pk_uuidv7<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    pk_uuid_default(name, Expr::cust("uuid_generate_v7()"))
}

/// Create a primary key column using [Uuid][sea_orm::sea_query::ColumnType::Uuid] as the column
/// type.
///
/// Provide a `default` expression in order to define how a default value is generated if no value
/// is not provided by the application.
pub fn pk_uuid_default<T, D>(name: T, default: D) -> ColumnDef
where
    T: IntoIden,
    D: Into<SimpleExpr>,
{
    pk_uuid(name).default(default).to_owned()
}
