//! Utility methods to create common column types in table create/alter statements.
//!
//! These utilities are similar to the ones provided by [SeaORM][sea_orm_migration::schema] and
//! [Loco](https://github.com/loco-rs/loco/blob/be7ead6e2503731aea252ed8dc6542d74f2c2e4f/src/schema.rs),
//! but with some minor differences. For example, our updated/created at timestamps include the
//! timezone, while SeaORM/Loco do not.

use sea_orm_migration::{prelude::*, schema::*};
use typed_builder::TypedBuilder;

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

/// Create a `BIGINT` primary key column with no default -- the application would need to provide
/// the value. Not exposed publicly; this should only be used internally as a utility method.
fn pk_bigint<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    big_integer(name).primary_key().to_owned()
}

/// Create an auto-incrementing primary key column using [BigInteger][sea_orm::sea_query::ColumnType::BigInteger]
/// as the column type.
#[deprecated(
    since = "0.5.10",
    note = "This creates a `BIGSERIAL` column, which is not recommended. Use `pk_bigint_identity` instead to create an IDENTITY column, as recommended [here](https://wiki.postgresql.org/wiki/Don%27t_Do_This#Don.27t_use_serial)."
)]
pub fn pk_bigint_auto<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    pk_bigint(name).auto_increment().to_owned()
}

/// Create an auto-incrementing primary key column using [BigInteger][sea_orm::sea_query::ColumnType::BigInteger]
/// as the column type. This creates an `IDENTITY` column instead of a `BIGSERIAL` as recommended
/// [here](https://wiki.postgresql.org/wiki/Don%27t_Do_This#Don.27t_use_serial).
///
/// See also: <https://www.postgresql.org/docs/17/ddl-identity-columns.html>
pub fn pk_bigint_identity<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    pk_bigint(name)
        .extra("GENERATED ALWAYS AS IDENTITY")
        .to_owned()
}

/// Configuration options for creating an `IDENTITY` column.
#[derive(TypedBuilder)]
pub struct IdentityOptions {
    /// If `true`, will add `ALWAYS` to the column definition, which will prevent the application
    /// from providing a value for the column. If the application needs to be able to set the
    /// value of the column, set to `false`.
    ///
    /// See: <https://www.postgresql.org/docs/17/ddl-identity-columns.html>
    #[builder(default = true)]
    always: bool,

    /// The value for the `START WITH` option of the `IDENTITY` sequence.
    ///
    /// See: <https://www.postgresql.org/docs/current/sql-createsequence.html>
    #[builder(default = 1)]
    start: i64,

    /// The value for the `INCREMENT BY` option of the `IDENTITY` sequence.
    ///
    /// See: <https://www.postgresql.org/docs/current/sql-createsequence.html>
    #[builder(default = 1)]
    increment: i64,
}

/// Same as [pk_bigint_identity], except allows configuring the IDENTITY column with the
/// given [IdentityOptions].
pub fn pk_bigint_identity_options<T>(name: T, options: IdentityOptions) -> ColumnDef
where
    T: IntoIden,
{
    let always = if options.always { " ALWAYS" } else { "" };
    let generated = format!(
        "GENERATED{} AS IDENTITY (START WITH {} INCREMENT BY {})",
        always, options.start, options.increment
    );
    pk_bigint(name).extra(generated).to_owned()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::assert_snapshot;
    use rstest::{fixture, rstest};

    #[derive(DeriveIden)]
    pub(crate) enum Foo {
        Table,
        Bar,
    }

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn table_stmt() -> TableCreateStatement {
        Table::create().table(Foo::Table).to_owned()
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn table() {
        let statement = super::table(Foo::Table);

        assert_snapshot!(statement.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn timestamps(_case: TestCase, table_stmt: TableCreateStatement) {
        let table_stmt = super::timestamps(table_stmt);

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_bigint(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(super::pk_bigint(Foo::Bar));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_bigint_auto(_case: TestCase, mut table_stmt: TableCreateStatement) {
        #[allow(deprecated)]
        table_stmt.col(super::pk_bigint_auto(Foo::Bar));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_bigint_identity(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(super::pk_bigint_identity(Foo::Bar));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[case(true, 1, 1)]
    #[case(false, 1, 1)]
    #[case(true, -100, 1)]
    #[case(true, 0, 1)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_bigint_identity_options(
        _case: TestCase,
        mut table_stmt: TableCreateStatement,
        #[case] always: bool,
        #[case] start: i64,
        #[case] increment: i64,
    ) {
        let options = IdentityOptions::builder()
            .always(always)
            .start(start)
            .increment(increment)
            .build();
        table_stmt.col(super::pk_bigint_identity_options(Foo::Bar, options));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_uuid(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(super::pk_uuid(Foo::Bar));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_uuidv4(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(super::pk_uuidv4(Foo::Bar));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_uuidv7(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(super::pk_uuidv7(Foo::Bar));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn pk_uuid_default(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(super::pk_uuid_default(
            Foo::Bar,
            Expr::cust("custom_uuid_fn()"),
        ));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }
}
