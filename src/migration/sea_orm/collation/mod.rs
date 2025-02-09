//! Utilities and migrations related to collations.

pub mod m20241022_065427_case_insensitive_collation;

use sea_orm::{DbBackend, Statement};
use sea_orm_migration::prelude::*;

/// Collations available from Roadster.
#[derive(DeriveIden)]
#[non_exhaustive]
pub enum Collation {
    /// The `default` collation. This comes included in Postgres.
    ///
    /// Note: This iden needs to be surrounded in quotes, at least in Postgres.
    Default,
    /// A case-insensitive collation.
    CaseInsensitive,
}

/// Wrapper around [`create_case_insensitive_collation`] to execute the returned [`Statement`], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::collation::exec_create_case_insensitive_collation;
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
///     async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_create_case_insensitive_collation(manager).await
///     }
/// #
/// #    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// }
/// ```
pub async fn exec_create_case_insensitive_collation(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    let statement = create_case_insensitive_collation(manager);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Create a case-insensitive collation.
///
/// See: <https://www.postgresql.org/docs/current/collation.html#COLLATION-NONDETERMINISTIC>
///
/// Note: Currently only supports Postgres. If another DB is used, will return [`None`].
pub fn create_case_insensitive_collation(manager: &SchemaManager<'_>) -> Option<Statement> {
    let backend = manager.get_database_backend();
    create_case_insensitive_collation_for_db_backend(backend)
}

fn create_case_insensitive_collation_for_db_backend(backend: DbBackend) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        Some(Statement::from_string(
            backend,
            format!(
                r#"CREATE COLLATION IF NOT EXISTS {} (
provider = icu,
locale = 'und-u-ks-level2',
deterministic = false
);
"#,
                Collation::CaseInsensitive.to_string()
            ),
        ))
    } else {
        None
    }
}

/// Wrapper around [`drop_case_insensitive_collation`] to execute the returned [`Statement`], if
/// present.
///
/// # Examples
/// ```rust
/// use roadster::migration::sea_orm::collation::exec_drop_case_insensitive_collation;
/// use sea_orm_migration::prelude::*;
///
/// #[derive(DeriveMigrationName)]
/// pub struct Migration;
///
/// #[async_trait::async_trait]
/// impl MigrationTrait for Migration {
/// #    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
/// #        unimplemented!()
/// #    }
/// #
///     async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
///         exec_drop_case_insensitive_collation(manager).await
///     }
/// }
/// ```
pub async fn exec_drop_case_insensitive_collation(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    let statement = drop_case_insensitive_collation(manager);
    if let Some(statement) = statement {
        manager.get_connection().execute(statement).await?;
    }

    Ok(())
}

/// Drop the case-insensitive collation that was previously created by [`create_case_insensitive_collation`].
///
/// Note: Currently only supports Postgres. If another DB is used, will return [None].
pub fn drop_case_insensitive_collation(manager: &SchemaManager<'_>) -> Option<Statement> {
    let backend = manager.get_database_backend();
    drop_case_insensitive_collation_for_db_backend(backend)
}

fn drop_case_insensitive_collation_for_db_backend(backend: DbBackend) -> Option<Statement> {
    if let DbBackend::Postgres = backend {
        Some(Statement::from_string(
            backend,
            format!(
                "DROP COLLATION IF EXISTS {};",
                Collation::CaseInsensitive.to_string()
            ),
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::snapshot::TestCase;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use sea_orm::DbBackend;

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
    fn create_case_insensitive_collation_for_db_backend(
        _case: TestCase,
        #[case] backend: DbBackend,
    ) {
        let statement = super::create_case_insensitive_collation_for_db_backend(backend);

        assert_debug_snapshot!(statement);
    }

    #[rstest]
    #[case(DbBackend::Postgres)]
    #[case(DbBackend::MySql)]
    #[case(DbBackend::Sqlite)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_case_insensitive_collation_for_db_backend(_case: TestCase, #[case] backend: DbBackend) {
        let statement = super::drop_case_insensitive_collation_for_db_backend(backend);

        assert_debug_snapshot!(statement);
    }
}
