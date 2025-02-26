use async_trait::async_trait;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use roadster::app::context::AppContext;
use roadster::app::App;
use roadster::db::migration::diesel::DieselMigrator;
use roadster::error::RoadsterResult;

pub struct MyApp;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[async_trait]
impl App<AppContext> for MyApp {
    type Cli = crate::cli::Cli;

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<AppContext> {
        Ok(context)
    }

    fn migrators(
        &self,
        _state: &AppContext,
    ) -> RoadsterResult<Vec<Box<dyn roadster::db::migration::Migrator<AppContext>>>> {
        Ok(vec![Box::new(
            DieselMigrator::<roadster::db::DieselPgConn>::new(MIGRATIONS),
        )])
    }
}
