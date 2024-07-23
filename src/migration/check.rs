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
