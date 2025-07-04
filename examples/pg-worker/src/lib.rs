use crate::api::http;
use crate::worker::example::{ExampleWorker, ExampleWorkerArgs};
use cron::Schedule;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::http::service::HttpService;
use roadster::service::worker::backend::pg::PgWorkerService;
use roadster::worker::backend::pg::processor::PgProcessor;
use roadster::worker::backend::pg::processor::builder::PeriodicArgs;
use std::str::FromStr;

pub mod api;
pub mod worker;

pub type App = RoadsterApp<AppContext>;

const BASE: &str = "/api";

pub fn build_app() -> App {
    let builder = RoadsterApp::builder();

    let builder = builder.state_provider(move |app_context| Ok(app_context));

    // Services can either be provided directly or via a provider callback. Each can be called
    // multiple times to register multiple services (however, registering duplicate services
    // will cause an error).
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
                registry.register_service(
                    PgWorkerService::builder()
                        .processor(
                            PgProcessor::builder(state)
                                .register(ExampleWorker)?
                                .register_periodic(
                                    ExampleWorker,
                                    PeriodicArgs::builder()
                                        .schedule(Schedule::from_str("* * * * * *")?)
                                        .args(
                                            ExampleWorkerArgs::builder()
                                                .foo("foo")
                                                .bar(111)
                                                .build(),
                                        )
                                        .build(),
                                )?
                                .build(),
                        )
                        .build(),
                )?;
                Ok(())
            })
        });

    builder.build()
}
