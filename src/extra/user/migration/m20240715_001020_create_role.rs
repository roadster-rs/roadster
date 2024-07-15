use crate::extra::migration::schema::{pk_uuid, table};
use itertools::Itertools;
use once_cell::sync::Lazy;
use sea_orm::sea_query::extension::postgres::{Type, TypeCreateStatement, TypeDropStatement};
use sea_orm_migration::prelude::*;
use sea_orm_migration::schema::custom;
use std::iter::{IntoIterator, Iterator};
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_type(create_role_enum()).await?;
        manager.create_table(create_table()).await?;

        for query in seed_roles()? {
            manager.exec_stmt(query).await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(drop_table()).await?;
        manager.drop_type(drop_role_enum()).await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
pub enum Role {
    Table,
    Id,
    Name,
}

// Define enums with the approach vs using an active enum in order to
// 1. support DBs other than postgres
// 2. avoid the unnecessary boilerplate of writing an active enum that will be a duplicate of the one
//    generated for the entity by seaorm
pub static ROLE_NAME_ENUM: Lazy<Alias> = Lazy::new(|| Alias::new("role_name"));
pub static ROLE_NAMES: Lazy<Vec<Alias>> = Lazy::new(|| {
    ["user", "super_admin"]
        .into_iter()
        .unique()
        .map(Alias::new)
        .collect()
});

fn create_role_enum() -> TypeCreateStatement {
    Type::create()
        .as_enum(ROLE_NAME_ENUM.clone())
        .values(ROLE_NAMES.clone())
        .to_owned()
}

fn create_table() -> TableCreateStatement {
    table(Role::Table)
        .col(pk_uuid(Role::Id))
        .col(custom(Role::Name.into_iden(), ROLE_NAME_ENUM.clone().into_iden()).unique_key())
        .to_owned()
}

fn seed_roles() -> Result<Vec<InsertStatement>, DbErr> {
    let mut statements = Vec::new();

    for role in ROLE_NAMES.iter() {
        let query = Query::insert()
            .into_table(Role::Table)
            .columns([Role::Id, Role::Name])
            .values([
                Uuid::now_v7().into(),
                Func::cast_as(role.to_string(), ROLE_NAME_ENUM.clone()).into(),
            ])
            .map_err(|err| DbErr::Migration(format!("{err}")))?
            .to_owned();
        statements.push(query);
    }

    Ok(statements)
}

fn drop_table() -> TableDropStatement {
    Table::drop().if_exists().table(Role::Table).to_owned()
}

fn drop_role_enum() -> TypeDropStatement {
    Type::drop()
        .if_exists()
        .name(ROLE_NAME_ENUM.clone())
        .to_owned()
}

#[cfg(test)]
mod tests {
    use crate::util::test_util::TestCaseConfig;
    use insta::{assert_debug_snapshot, assert_snapshot};
    use itertools::Itertools;
    use sea_orm::sea_query::PostgresQueryBuilder;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_role_enum() {
        let query = super::create_role_enum();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn create_table() {
        let query = super::create_table();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn seed_roles() {
        let _case = TestCaseConfig::builder().set_suffix(false).build();

        let queries = super::seed_roles().unwrap();

        let queries = queries
            .into_iter()
            .map(|query| query.to_string(PostgresQueryBuilder))
            .collect_vec();

        assert_debug_snapshot!(queries);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_role_enum() {
        let query = super::drop_role_enum();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn drop_table() {
        let query = super::drop_table();

        assert_snapshot!(query.to_string(PostgresQueryBuilder));
    }
}
