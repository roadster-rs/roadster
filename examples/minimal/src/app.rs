use aide::axum::ApiRouter;
use async_trait::async_trait;
use migration::Migrator;
use roadster::app::App as RoadsterApp;
use roadster::app_context::AppContext;
use roadster::config::app_config::AppConfig;
use roadster::controller::default_routes;
use roadster::worker::app_worker::AppWorker;
use roadster::worker::registry::WorkerRegistry;

use crate::app_state::AppState;
use crate::cli::AppCli;
use crate::controller;
use crate::worker::example::ExampleWorker;

const BASE: &str = "/api";

#[derive(Default)]
pub struct App;

#[async_trait]
impl RoadsterApp for App {
    type State = AppState;
    type Cli = AppCli;
    type M = Migrator;

    fn router(config: &AppConfig) -> ApiRouter<Self::State> {
        default_routes(BASE, config).merge(controller::routes(BASE))
    }

    async fn workers(
        registry: &mut WorkerRegistry<Self>,
        _context: &AppContext,
        state: &Self::State,
    ) -> anyhow::Result<()> {
        registry.register_app_worker(ExampleWorker::build(state));
        Ok(())
    }
}
