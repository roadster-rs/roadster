use async_trait::async_trait;
use migration::Migrator;
use roadster::app::App as RoadsterApp;
use roadster::app_context::AppContext;
use roadster::service::http::http_service_builder::HttpServiceBuilder;
use roadster::service::AppService;
use roadster::worker::app_worker::AppWorker;
use roadster::worker::registry::WorkerRegistry;
use std::vec;

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

    async fn workers(
        registry: &mut WorkerRegistry<Self>,
        _context: &AppContext,
        state: &Self::State,
    ) -> anyhow::Result<()> {
        registry.register_app_worker(ExampleWorker::build(state));
        Ok(())
    }

    async fn services(
        context: &AppContext,
        state: &Self::State,
    ) -> anyhow::Result<Vec<Box<dyn AppService<Self>>>> {
        let http_service = Box::new(
            HttpServiceBuilder::<Self>::new(BASE, context)
                .router(controller::routes(BASE))
                .build(context, state)?,
        );

        Ok(vec![http_service])
    }
}
