use super::*;
use insta::assert_debug_snapshot;
use itertools::Itertools;

#[test]
fn user_migrator_migrations() {
    let user_migrations = UserMigrator::migrations()
        .into_iter()
        .map(|migration| migration.name().to_string())
        .collect_vec();
    assert_debug_snapshot!(user_migrations);
}

#[test]
fn user_migrator_migrations_no_int_pk() {
    let user_migrations = UserMigrator::migrations()
        .into_iter()
        .map(|migration| migration.name().to_string())
        .collect_vec();

    assert!(!user_migrations.contains(
        &m20240714_203550_create_user_table_int_pk::Migration::default()
            .name()
            .to_string()
    ))
}
