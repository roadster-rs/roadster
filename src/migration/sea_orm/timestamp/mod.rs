//! Utilities and migrations related to timestamp fields.

use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

pub mod m20240723_201404_add_update_timestamp_function;

/// Timestamp related fields.
#[derive(DeriveIden)]
#[non_exhaustive]
pub enum Timestamps {
    /// When the row was created. When used with the [`crate::migration::sea_orm::schema::timestamps`] method, will default to
    /// the current timestamp (with timezone).
    CreatedAt,
    /// When the row was updated. When used with the [`crate::migration::sea_orm::schema::timestamps`] method, will be initially set to
    /// the current timestamp (with timezone).
    ///
    /// To automatically update the value for a row whenever the row is updated, include the
    /// [`m20240723_201404_add_update_timestamp_function::Migration`]
    /// in your [`MigratorTrait`] implementation, along with a [`MigrationTrait`] for your table
    /// that add a trigger to update the column. Helper methods are provided for this in
    /// the [`crate::migration::sea_orm::timestamp`] module. Specifically, see:
    /// - [`exec_create_update_timestamp_trigger`]
    /// - [`exec_drop_update_timestamp_trigger`]
    ///
    /// Note that the auto-updates mentioned above are currently only supported on Postgres. If
    /// an app is using a different DB, it will need to manually update the timestamp when updating
    /// a row.
    UpdatedAt,
}

/// Wrapper around [`create_update_timestamp_function`] to execute the returned [`Statement`], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::timestamp::Timestamps;
/// use roadster::migration::sea_orm::timestamp::exec_create_update_timestamp_function;
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// const COLUMN: Timestamps = Timestamps::UpdatedAt;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
///     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_create_update_timestamp_function(manager, COLUMN).await
///     }
/// #
/// #    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// }
/// ```
pub async fn exec_create_update_timestamp_function<C: IntoIden>(
    manager: &SchemaManager<'_>,
    timestamp_column: C,
) -> Result<(), DbErr> {
    let statement = create_update_timestamp_function(manager, timestamp_column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Wrapper around [`create_update_timestamp_function_dep_column`] to execute the returned
/// [`Statement`], if present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::timestamp::Timestamps;
/// use roadster::migration::sea_orm::timestamp::{exec_create_update_timestamp_function_dep_column};
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// #[derive(DeriveIden)]
/// pub(crate) enum User {
///     Table,
///     Name,
///     NameUpdatedAt
/// }
///
/// const TIMESTAMP_COLUMN: User = User::NameUpdatedAt;
/// const DEPENDENT_COLUMN: User = User::Name;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
///     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_create_update_timestamp_function_dep_column(
///             manager,
///             TIMESTAMP_COLUMN,
///             DEPENDENT_COLUMN,
///         )
///         .await
///     }
/// #
/// #    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// }
/// ```
pub async fn exec_create_update_timestamp_function_dep_column<C: IntoIden, D: IntoIden>(
    manager: &SchemaManager<'_>,
    timestamp_column: C,
    dep_column: D,
) -> Result<(), DbErr> {
    let statement =
        create_update_timestamp_function_dep_column(manager, timestamp_column, dep_column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Create a SQL function to update a timestamp column with the current timestamp. Returns
/// a [`Statement`] containing the SQL instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
pub fn create_update_timestamp_function<C: IntoIden>(
    manager: &SchemaManager<'_>,
    timestamp_column: C,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    create_update_timestamp_function_for_db_backend(backend, timestamp_column)
}

/// Create a SQL function to update a timestamp column with the current timestamp, but only
/// if the provided dependent column is modified. Returns a [`Statement`] containing the SQL
/// instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
pub fn create_update_timestamp_function_dep_column<C: IntoIden, D: IntoIden>(
    manager: &SchemaManager<'_>,
    timestamp_column: C,
    dep_column: D,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    create_update_timestamp_function_dep_column_for_db_backend(
        backend,
        timestamp_column,
        dep_column,
    )
}

/// Create a SQL function to update a timestamp column with the current timestamp. Returns
/// a [`Statement`] containing the SQL instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
fn create_update_timestamp_function_for_db_backend<C: IntoIden>(
    backend: DbBackend,
    timestamp_column: C,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let FnQueryStrings {
            timestamp_column,
            fn_call,
            ..
        } = FnQueryStrings::new::<_, C>(timestamp_column, None);

        let statement = Statement::from_string(
            backend,
            format!(
                r#"
CREATE OR REPLACE FUNCTION {fn_call} RETURNS TRIGGER AS $$
BEGIN
    NEW.{timestamp_column} = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';
"#
            ),
        );
        Some(statement)
    } else {
        None
    }
}

/// Create a SQL function to update a timestamp column with the current timestamp, but only
/// if the provided dependent column is modified. Returns a [`Statement`] containing the SQL
/// instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
fn create_update_timestamp_function_dep_column_for_db_backend<C: IntoIden, D: IntoIden>(
    backend: DbBackend,
    timestamp_column: C,
    dep_column: D,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let FnQueryStrings {
            timestamp_column,
            dep_column,
            fn_call,
            ..
        } = FnQueryStrings::new(timestamp_column, Some(dep_column));
        #[allow(clippy::expect_used)]
        let dep_column = dep_column.expect("Dependent column should be present");

        let statement = Statement::from_string(
            backend,
            format!(
                r#"
CREATE OR REPLACE FUNCTION {fn_call} RETURNS TRIGGER AS $$
BEGIN
    IF OLD.{dep_column} IS DISTINCT FROM NEW.{dep_column} THEN
        NEW.{timestamp_column} = NOW();
    END IF;
    RETURN NEW;
END;
$$ language 'plpgsql';
"#
            ),
        );
        Some(statement)
    } else {
        None
    }
}

/// Wrapper around [`drop_update_timestamp_function`] to execute the returned [`Statement`], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::timestamp::Timestamps;
/// use roadster::migration::sea_orm::timestamp::exec_drop_update_timestamp_function;
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// const COLUMN: Timestamps = Timestamps::UpdatedAt;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
/// #    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// #
///     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_drop_update_timestamp_function(manager, COLUMN).await
///     }
/// }
/// ```
pub async fn exec_drop_update_timestamp_function<C: IntoIden>(
    manager: &SchemaManager<'_>,
    timestamp_column: C,
) -> Result<(), DbErr> {
    let statement = drop_update_timestamp_function(manager, timestamp_column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Drop a SQL function that was previously created by [`create_update_timestamp_function`]
/// or [`create_update_timestamp_function`]. Returns a [`Statement`] containing the SQL
/// instructions to drop the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
pub fn drop_update_timestamp_function<C: IntoIden>(
    manager: &SchemaManager<'_>,
    timestamp_column: C,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    drop_update_timestamp_function_for_db_backend(backend, timestamp_column)
}

fn drop_update_timestamp_function_for_db_backend<C: IntoIden>(
    backend: DbBackend,
    timestamp_column: C,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let FnQueryStrings { fn_name, .. } = FnQueryStrings::new::<_, C>(timestamp_column, None);

        let statement =
            Statement::from_string(backend, format!(r#"DROP FUNCTION IF EXISTS {fn_name};"#));
        Some(statement)
    } else {
        None
    }
}

/// Wrapper around [`create_update_timestamp_trigger`] to execute the returned [`Statement`], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::timestamp::Timestamps;
/// use roadster::migration::sea_orm::timestamp::exec_create_update_timestamp_trigger;
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// const TABLE: Foo = Foo::Table;
/// const COLUMN: Timestamps = Timestamps::UpdatedAt;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
///     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_create_update_timestamp_trigger(manager, TABLE, COLUMN).await
///     }
/// #
/// #    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// }
///
/// #[derive(DeriveIden)]
/// pub(crate) enum Foo {
///     Table,
///     // ...
/// }
/// ```
pub async fn exec_create_update_timestamp_trigger<T: IntoTableRef + IntoIden, C: IntoIden>(
    manager: &SchemaManager<'_>,
    table: T,
    timestamp_column: C,
) -> Result<(), DbErr> {
    let statement = create_update_timestamp_trigger(manager, table, timestamp_column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Create a SQL trigger to automatically update a timestamp column of a row whenever the row is
/// updated. Depends on the function created by [`create_update_timestamp_function`].
/// Returns a [`Statement`] containing the SQL instructions to create the trigger.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
pub fn create_update_timestamp_trigger<T: IntoTableRef + IntoIden, C: IntoIden>(
    manager: &SchemaManager<'_>,
    table: T,
    timestamp_column: C,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    create_update_timestamp_trigger_for_db_backend(backend, table, timestamp_column)
}

fn create_update_timestamp_trigger_for_db_backend<T: IntoTableRef + IntoIden, C: IntoIden>(
    backend: DbBackend,
    table: T,
    timestamp_column: C,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let TriggerQueryNames {
            fn_query_strings: FnQueryStrings { fn_call, .. },
            table,
            trigger_name,
            ..
        } = TriggerQueryNames::new::<_, _, T>(table, timestamp_column, None);

        let statement = Statement::from_string(
            backend,
            format!(
                r#"
CREATE TRIGGER {trigger_name} BEFORE UPDATE
ON {table}
FOR EACH ROW
EXECUTE PROCEDURE {fn_call};
"#
            ),
        );

        Some(statement)
    } else {
        None
    }
}

/// Wrapper around [`drop_update_timestamp_trigger`] to execute the returned [`Statement`], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::timestamp::Timestamps;
/// use roadster::migration::sea_orm::timestamp::exec_drop_update_timestamp_trigger;
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// const TABLE: Foo = Foo::Table;
/// const COLUMN: Timestamps = Timestamps::UpdatedAt;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
/// #    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// #
///     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_drop_update_timestamp_trigger(manager, TABLE, COLUMN).await
///     }
/// }
///
/// #[derive(DeriveIden)]
/// pub(crate) enum Foo {
///     Table,
///     // ...
/// }
/// ```
pub async fn exec_drop_update_timestamp_trigger<T: IntoTableRef + IntoIden, C: IntoIden>(
    manager: &SchemaManager<'_>,
    table: T,
    timestamp_column: C,
) -> Result<(), DbErr> {
    let statement = drop_update_timestamp_trigger(manager, table, timestamp_column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Drop a SQL trigger that was previously created by [`create_update_timestamp_trigger`].
/// Returns a [`Statement`] containing the SQL instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
pub fn drop_update_timestamp_trigger<T: IntoTableRef + IntoIden, C: IntoIden>(
    manager: &SchemaManager<'_>,
    table: T,
    timestamp_column: C,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    drop_update_timestamp_trigger_for_db_backend(backend, table, timestamp_column)
}

fn drop_update_timestamp_trigger_for_db_backend<T: IntoTableRef + IntoIden, C: IntoIden>(
    backend: DbBackend,
    table: T,
    timestamp_column: C,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let TriggerQueryNames {
            table,
            trigger_name,
            ..
        } = TriggerQueryNames::new::<_, _, T>(table, timestamp_column, None);

        let statement = Statement::from_string(
            backend,
            format!(r#"DROP TRIGGER IF EXISTS {trigger_name} ON {table};"#),
        );
        Some(statement)
    } else {
        None
    }
}

#[derive(Debug)]
struct FnQueryStrings {
    timestamp_column: String,
    dep_column: Option<String>,
    fn_name: String,
    fn_call: String,
}

#[derive(Debug)]
struct TriggerQueryNames {
    fn_query_strings: FnQueryStrings,
    table: String,
    trigger_name: String,
}

impl FnQueryStrings {
    fn new<C: IntoIden, D: IntoIden>(timestamp_column: C, dep_column: Option<D>) -> Self {
        let timestamp_column = timestamp_column.into_iden().to_string();
        let dep_column = dep_column.map(|c| c.into_iden().to_string());
        let fn_name = update_timestamp_fn_name(&timestamp_column);
        let fn_call = format!("{fn_name}()");

        Self {
            timestamp_column,
            dep_column,
            fn_name,
            fn_call,
        }
    }
}

impl TriggerQueryNames {
    fn new<T: IntoTableRef + IntoIden, C: IntoIden, D: IntoIden>(
        table: T,
        timestamp_column: C,
        dep_column: Option<D>,
    ) -> Self {
        let fn_query_strings = FnQueryStrings::new(timestamp_column, dep_column);
        let table = table.into_iden().to_string();
        let trigger_name = trigger_name(&table, &fn_query_strings.fn_name);

        Self {
            fn_query_strings,
            table: format!("public.{table}"),
            trigger_name,
        }
    }
}

fn update_timestamp_fn_name(column: &str) -> String {
    format!("update_timestamp_{column}")
}

fn trigger_name(table: &str, fn_name: &str) -> String {
    format!("{table}_{fn_name}")
}

#[cfg(test)]
mod tests {
    use crate::migration::sea_orm::timestamp::{FnQueryStrings, TriggerQueryNames};
    use crate::testing::snapshot::TestCase;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use sea_orm::DbBackend;
    use sea_orm_migration::prelude::*;

    #[derive(DeriveIden)]
    enum Foo {
        Table,
        UpdatedAt,
        Password,
        PasswordUpdatedAt,
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn add_update_timestamp_trigger(_case: TestCase, #[case] backend: DbBackend) {
        let statement = super::create_update_timestamp_trigger_for_db_backend(
            backend,
            Foo::Table,
            Foo::UpdatedAt,
        );

        assert_debug_snapshot!(statement);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_update_timestamp_trigger(_case: TestCase, #[case] backend: DbBackend) {
        let statement = super::drop_update_timestamp_trigger_for_db_backend(
            backend,
            Foo::Table,
            Foo::UpdatedAt,
        );

        assert_debug_snapshot!(statement);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn add_update_timestamp_function(_case: TestCase, #[case] backend: DbBackend) {
        let statement =
            super::create_update_timestamp_function_for_db_backend(backend, Foo::UpdatedAt);

        assert_debug_snapshot!(statement);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn add_update_timestamp_function_dep_column(_case: TestCase, #[case] backend: DbBackend) {
        let statement = super::create_update_timestamp_function_dep_column_for_db_backend(
            backend,
            Foo::PasswordUpdatedAt,
            Foo::Password,
        );

        assert_debug_snapshot!(statement);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_update_timestamp_function(_case: TestCase, #[case] backend: DbBackend) {
        let statement =
            super::drop_update_timestamp_function_for_db_backend(backend, Foo::UpdatedAt);

        assert_debug_snapshot!(statement);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fn_query_strings() {
        let fn_query_strings = FnQueryStrings::new::<_, Foo>(Foo::UpdatedAt, None);
        assert_debug_snapshot!(fn_query_strings);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn trigger_query_strings() {
        let trigger_query_strings =
            TriggerQueryNames::new::<_, _, Foo>(Foo::Table, Foo::UpdatedAt, None);
        assert_debug_snapshot!(trigger_query_strings);
    }
}
