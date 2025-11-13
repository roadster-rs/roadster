use crate::migrator::Migrator;
use roadster::app::App;
use roadster::app::context::AppContext;
use roadster::config::AppConfig;
use roadster::db::migration::registry::MigratorRegistry;
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
        registry: &mut MigratorRegistry<AppContext>,
        _state: &AppContext,
    ) -> Result<(), Self::Error> {
        registry.register_sea_orm_migrator(Migrator)?;
        Ok(())
    }
}
