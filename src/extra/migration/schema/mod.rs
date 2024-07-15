use sea_orm::sea_query::{ColumnDef, Expr, IntoIden, SimpleExpr, TableCreateStatement};
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::{big_integer, timestamp_with_time_zone};

#[derive(DeriveIden)]
enum GeneralIds {
    CreatedAt,
    UpdatedAt,
}

/// Create a table if it does not exist yet and add some default columns
/// (e.g., create/update timestamps).
pub fn table<T: IntoIden + 'static>(name: T) -> TableCreateStatement {
    timestamps(Table::create().table(name).if_not_exists().take())
}

/// Add "timestamp with time zone" columns (`CreatedAt` and `UpdatedAt`) to a table.
/// The default for each column is the current timestamp.
pub fn timestamps(mut table: TableCreateStatement) -> TableCreateStatement {
    table
        .col(timestamp_with_time_zone(GeneralIds::CreatedAt).default(Expr::current_timestamp()))
        .col(timestamp_with_time_zone(GeneralIds::UpdatedAt).default(Expr::current_timestamp()))
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
/// type. A new v4 UUID will be generated as the default if no value is provided by the application.
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
