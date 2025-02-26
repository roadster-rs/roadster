use crate::migrator::Migrator;
use roadster::app::App;
use roadster::app::context::AppContext;
use roadster::db::migration::sea_orm::SeaOrmMigrator;
use roadster::error::RoadsterResult;
use sea_orm::prelude::async_trait::async_trait;

pub struct MyApp;

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
        Ok(vec![Box::new(SeaOrmMigrator::new(Migrator))])
    }
}
