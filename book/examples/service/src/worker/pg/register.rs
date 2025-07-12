use crate::worker::pg::worker::ExampleWorker;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::worker::PgWorkerService;
use roadster::worker::{PeriodicArgs, PgProcessor};
use std::str::FromStr;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(Ok)
        // Register the Postgres worker service
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                let processor = PgProcessor::builder(state)
                    // Register the `ExampleWorker` with the Postgres worker service
                    .register(ExampleWorker)?
                    // Example of registering the `ExampleWorker` to run as a periodic cron job
                    .register_periodic(
                        ExampleWorker,
                        PeriodicArgs::builder()
                            .args("Periodic example args".to_string())
                            .schedule(cron::Schedule::from_str("* * * * * *")?)
                            .build(),
                    )?
                    .build()
                    .await?;
                registry
                    .register_service(PgWorkerService::builder().processor(processor).build())?;
                Ok(())
            })
        })
        .build()
}
