use sea_orm_migration::prelude::*;

pub mod m20240714_203551_create_user_table;
mod m20240715_001020_create_role;

pub struct UserMigrator;

#[async_trait::async_trait]
impl MigratorTrait for UserMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240715_001020_create_role::Migration),
            Box::new(m20240714_203551_create_user_table::Migration),
        ]
    }
}
