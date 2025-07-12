use crate::api::http;
use crate::worker::example::ExampleWorker;
use crate::worker::example_periodic::{ExamplePeriodicWorker, ExamplePeriodicWorkerArgs};
use cron::Schedule;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::http::service::HttpService;
use roadster::service::worker::PgWorkerService;
use roadster::worker::PeriodicArgs;
use roadster::worker::backend::pg::processor::PgProcessor;
use std::str::FromStr;

pub mod api;
pub mod worker;

pub type App = RoadsterApp<AppContext>;

const BASE: &str = "/api";

pub fn build_app() -> App {
    let builder = RoadsterApp::builder();

    let builder = builder.state_provider(Ok);

    let builder = builder
        .add_service_provider(|registry, state| {
            Box::pin(async {
                registry
                    .register_builder(
                        HttpService::builder(Some(BASE), state).api_router(http::routes(BASE)),
                    )
                    .await?;
                Ok(())
            })
        })
        .add_service_provider(|registry, state| {
            Box::pin(async {
                let processor = PgProcessor::builder(state)
                    .register(ExampleWorker)?
                    .register_periodic(
                        ExamplePeriodicWorker,
                        PeriodicArgs::builder()
                            .schedule(Schedule::from_str("*/10 * * * * *")?)
                            .args(ExamplePeriodicWorkerArgs::builder().a(111).build())
                            .build(),
                    )?
                    .build()
                    .await?;

                registry
                    .register_service(PgWorkerService::builder().processor(processor).build())?;

                Ok(())
            })
        });

    builder.build()
}
