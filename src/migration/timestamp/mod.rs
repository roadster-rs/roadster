//! Utilities and migrations related to timestamp fields.

use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

pub mod m20240723_201404_add_update_timestamp_function;

/// Wrapper around [create_update_timestamp_function] to execute the returned [Statement], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::schema::Timestamps;
/// use roadster::migration::timestamp::exec_create_update_timestamp_function;
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
/// #        todo!()
/// #    }
/// }
/// ```
pub async fn exec_create_update_timestamp_function<T: IntoIden>(
    manager: &SchemaManager<'_>,
    column: T,
) -> Result<(), DbErr> {
    let statement = create_update_timestamp_function(manager, column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Create a SQL function to update a timestamp column with the current timestamp. Returns
/// a [Statement] containing the SQL instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [None].
pub fn create_update_timestamp_function<T: IntoIden>(
    manager: &SchemaManager<'_>,
    column: T,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    create_update_timestamp_function_for_db_backend(backend, column)
}

fn create_update_timestamp_function_for_db_backend<T: IntoIden>(
    backend: DbBackend,
    column: T,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let FnQueryStrings {
            column, fn_call, ..
        } = FnQueryStrings::new(column);

        let statement = Statement::from_string(
            backend,
            format!(
                r#"
CREATE OR REPLACE FUNCTION {fn_call} RETURNS TRIGGER AS $$
BEGIN
    NEW.{column} = NOW();
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

/// Wrapper around [drop_update_timestamp_function] to execute the returned [Statement], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::schema::Timestamps;
/// use roadster::migration::timestamp::exec_drop_update_timestamp_function;
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
/// #        todo!()
/// #    }
/// #
///     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_drop_update_timestamp_function(manager, COLUMN).await
///     }
/// }
/// ```
pub async fn exec_drop_update_timestamp_function<T: IntoIden>(
    manager: &SchemaManager<'_>,
    column: T,
) -> Result<(), DbErr> {
    let statement = drop_update_timestamp_function(manager, column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Drop a SQL function that was previously created by [create_update_timestamp_function].
/// Returns a [Statement] containing the SQL instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [None].
pub fn drop_update_timestamp_function<T: IntoIden>(
    manager: &SchemaManager<'_>,
    column: T,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    drop_update_timestamp_function_for_db_backend(backend, column)
}

fn drop_update_timestamp_function_for_db_backend<T: IntoIden>(
    backend: DbBackend,
    column: T,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let FnQueryStrings { fn_name, .. } = FnQueryStrings::new(column);

        let statement =
            Statement::from_string(backend, format!(r#"DROP FUNCTION IF EXISTS {fn_name};"#));
        Some(statement)
    } else {
        None
    }
}

/// Wrapper around [create_update_timestamp_trigger] to execute the returned [Statement], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::schema::Timestamps;
/// use roadster::migration::timestamp::exec_create_update_timestamp_trigger;
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
/// #        todo!()
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
    column: C,
) -> Result<(), DbErr> {
    let statement = create_update_timestamp_trigger(manager, table, column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Create a SQL trigger to automatically update a timestamp column of a row whenever the row is
/// updated. Depends on the function created by [create_update_timestamp_function].
/// Returns a [Statement] containing the SQL instructions to create the trigger.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [None].
pub fn create_update_timestamp_trigger<T: IntoTableRef + IntoIden, C: IntoIden>(
    manager: &SchemaManager<'_>,
    table: T,
    column: C,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    create_update_timestamp_trigger_for_db_backend(backend, table, column)
}

fn create_update_timestamp_trigger_for_db_backend<T: IntoTableRef + IntoIden, C: IntoIden>(
    backend: DbBackend,
    table: T,
    column: C,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let TriggerQueryNames {
            fn_query_strings: FnQueryStrings { fn_call, .. },
            table,
            trigger_name,
        } = TriggerQueryNames::new(table, column);

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

/// Wrapper around [drop_update_timestamp_trigger] to execute the returned [Statement], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::schema::Timestamps;
/// use roadster::migration::timestamp::exec_drop_update_timestamp_trigger;
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
/// #        todo!()
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
    column: C,
) -> Result<(), DbErr> {
    let statement = drop_update_timestamp_trigger(manager, table, column);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Drop a SQL trigger that was previously created by [create_update_timestamp_trigger].
/// Returns a [Statement] containing the SQL instructions to create the function.
///
/// Note: Currently only supports Postgres. If another DB is used, will return [None].
pub fn drop_update_timestamp_trigger<T: IntoTableRef + IntoIden, C: IntoIden>(
    manager: &SchemaManager<'_>,
    table: T,
    column: C,
) -> Option<Statement> {
    let backend = manager.get_database_backend();
    drop_update_timestamp_trigger_for_db_backend(backend, table, column)
}

fn drop_update_timestamp_trigger_for_db_backend<T: IntoTableRef + IntoIden, C: IntoIden>(
    backend: DbBackend,
    table: T,
    column: C,
) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        let TriggerQueryNames {
            table,
            trigger_name,
            ..
        } = TriggerQueryNames::new(table, column);

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
    column: String,
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
    fn new<C: IntoIden>(column: C) -> Self {
        let column = column.into_iden().to_string();
        let fn_name = update_timestamp_fn_name(&column);
        let fn_call = format!("{fn_name}()");

        Self {
            column,
            fn_name,
            fn_call,
        }
    }
}

impl TriggerQueryNames {
    fn new<T: IntoTableRef + IntoIden, C: IntoIden>(table: T, column: C) -> Self {
        let fn_query_strings = FnQueryStrings::new(column);
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
    use crate::migration::timestamp::{FnQueryStrings, TriggerQueryNames};
    use crate::testing::snapshot::TestCase;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use sea_orm::DbBackend;
    use sea_orm_migration::prelude::*;

    #[derive(DeriveIden)]
    enum Foo {
        Table,
        UpdatedAt,
    }

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
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
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn add_update_timestamp_function(_case: TestCase, #[case] backend: DbBackend) {
        let statement =
            super::create_update_timestamp_function_for_db_backend(backend, Foo::UpdatedAt);

        assert_debug_snapshot!(statement);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_update_timestamp_function(_case: TestCase, #[case] backend: DbBackend) {
        let statement =
            super::drop_update_timestamp_function_for_db_backend(backend, Foo::UpdatedAt);

        assert_debug_snapshot!(statement);
    }

    #[test]
    fn fn_query_strings() {
        let fn_query_strings = FnQueryStrings::new(Foo::UpdatedAt);
        assert_debug_snapshot!(fn_query_strings);
    }

    #[test]
    fn trigger_query_strings() {
        let trigger_query_strings = TriggerQueryNames::new(Foo::Table, Foo::UpdatedAt);
        assert_debug_snapshot!(trigger_query_strings);
    }
}
