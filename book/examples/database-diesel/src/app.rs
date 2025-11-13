use async_trait::async_trait;
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use roadster::app::App;
use roadster::app::context::AppContext;
use roadster::config::AppConfig;
use roadster::db::migration::registry::MigratorRegistry;

pub struct MyApp;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

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
        registry: &mut MigratorRegistry<AppContext>,
    ) -> Result<(), Self::Error> {
        registry.register_diesel_migrator::<roadster::db::DieselPgConn>(MIGRATIONS)?;
        Ok(())
    }
}
