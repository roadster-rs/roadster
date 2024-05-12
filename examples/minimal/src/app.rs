use async_trait::async_trait;
use migration::Migrator;
use roadster::app::App as RoadsterApp;
use roadster::app_context::AppContext;
use roadster::service::http::service::HttpService;
use roadster::service::registry::ServiceRegistry;
use roadster::service::worker::sidekiq::app_worker::AppWorker;
use roadster::service::worker::sidekiq::service::SidekiqWorkerService;

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

    async fn with_state(_context: &AppContext) -> anyhow::Result<Self::State> {
        Ok(())
    }

    async fn services(
        registry: &mut ServiceRegistry<Self>,
        context: &AppContext<Self::State>,
    ) -> anyhow::Result<()> {
        registry
            .register_builder(HttpService::builder(BASE, context).router(controller::routes(BASE)))
            .await?;

        registry
            .register_builder(
                SidekiqWorkerService::builder(context)
                    .await?
                    .register_app_worker(ExampleWorker::build(context))?,
            )
            .await?;

        Ok(())
    }
}
