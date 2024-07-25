use crate::migration::check::str_not_empty;
use crate::migration::schema::{pk_bigint_auto, pk_uuid, table};
use crate::migration::user::User;
use sea_orm_migration::{prelude::*, schema::*};

pub(crate) fn create_table_uuid_pk() -> TableCreateStatement {
    create_table(pk_uuid(User::Id))
}

pub(crate) fn create_table_int_pk() -> TableCreateStatement {
    create_table(pk_bigint_auto(User::Id))
}

pub(crate) fn create_table(pk_col: ColumnDef) -> TableCreateStatement {
    table(User::Table)
        .col(pk_col)
        .col(string(User::Name).check(str_not_empty(User::Name)))
        .col(string_uniq(User::Username).check(str_not_empty(User::Username)))
        .col(string_uniq(User::Email).check(str_not_empty(User::Email)))
        .col(string(User::Password))
        .to_owned()
}

pub(crate) fn drop_table() -> TableDropStatement {
    Table::drop().if_exists().table(User::Table).to_owned()
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_table_uuid_pk() {
        let query = super::create_table_uuid_pk();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_table_int_pk() {
        let query = super::create_table_int_pk();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_table() {
        let query = super::drop_table();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }
}
