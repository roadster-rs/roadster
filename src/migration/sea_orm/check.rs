//! Utility methods to add check constraints to columns.

use sea_orm_migration::prelude::*;

/// Expression to check that a string column value is not empty.
pub fn str_not_empty<T>(name: T) -> SimpleExpr
where
    T: IntoIden + 'static,
{
    str_len_gt(name, 0)
}

/// Expression to check that a string column value's length is greater than the provided value.
pub fn str_len_gt<T>(name: T, len: u64) -> SimpleExpr
where
    T: IntoIden + 'static,
{
    Expr::expr(Func::char_length(Expr::col(name))).gt(len)
}

/// Expression to check that a string column value's length is greater than or equal to the
/// provided value.
pub fn str_len_gte<T>(name: T, len: u64) -> SimpleExpr
where
    T: IntoIden + 'static,
{
    Expr::expr(Func::char_length(Expr::col(name))).gte(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::assert_snapshot;
    use rstest::{fixture, rstest};
    use sea_orm_migration::schema::string;

    #[derive(DeriveIden)]
    pub(crate) enum Foo {
        Table,
        Bar,
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn table_stmt() -> TableCreateStatement {
        Table::create().table(Foo::Table).to_owned()
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn str_not_empty(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(string(Foo::Bar).check(super::str_not_empty(Foo::Bar)));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn str_len_gt(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(string(Foo::Bar).check(super::str_len_gt(Foo::Bar, 1)));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn str_len_gte(_case: TestCase, mut table_stmt: TableCreateStatement) {
        table_stmt.col(string(Foo::Bar).check(super::str_len_gte(Foo::Bar, 1)));

        assert_snapshot!(table_stmt.to_string(PostgresQueryBuilder));
    }
}
