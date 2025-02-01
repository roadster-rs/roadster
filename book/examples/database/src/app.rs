use roadster::app::context::AppContext;
use roadster::app::App;
use roadster::error::RoadsterResult;
use sea_orm::prelude::async_trait::async_trait;

pub struct MyApp;

#[async_trait]
impl App<AppContext> for MyApp {
    type Cli = crate::cli::Cli;
    type M = crate::migrator::Migrator;

    async fn provide_state(&self, context: AppContext) -> RoadsterResult<AppContext> {
        Ok(context)
    }
}
