use crate::migrator::Migrator;
use roadster::app::App;
use roadster::app::context::AppContext;
use roadster::config::AppConfig;
use roadster::db::migration::sea_orm::SeaOrmMigrator;
use sea_orm::prelude::async_trait::async_trait;

pub struct MyApp;

#[async_trait]
impl App<AppContext> for MyApp {
    type Error = roadster::error::Error;
    type Cli = crate::cli::Cli;

    async fn provide_state(&self, context: AppContext) -> Result<AppContext, Self::Error> {
        Ok(context)
    }

    fn init_tracing(&self, config: &AppConfig) -> Result<(), Self::Error> {
        roadster::tracing::init_tracing(config, &self.metadata(config)?)?;

        Ok(())
    }

    fn migrators(
        &self,
        _state: &AppContext,
    ) -> Result<Vec<Box<dyn roadster::db::migration::Migrator<AppContext>>>, Self::Error> {
        Ok(vec![Box::new(SeaOrmMigrator::new(Migrator))])
    }
}
