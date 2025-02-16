use sea_orm_migration::{MigrationTrait, MigratorTrait};

pub struct EmptyMigrator;

impl MigratorTrait for EmptyMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        Default::default()
    }
}
