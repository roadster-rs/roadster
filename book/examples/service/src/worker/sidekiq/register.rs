use crate::worker::sidekiq::worker::ExampleWorker;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::service::worker::SidekiqWorkerService;
use roadster::worker::{PeriodicArgs, SidekiqProcessor};
use std::str::FromStr;

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(Ok)
        // Register the Sidekiq worker service
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                let processor = SidekiqProcessor::builder(state)
                    // Register the `ExampleWorker` with the sidekiq service
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
                registry.register_service(
                    SidekiqWorkerService::builder().processor(processor).build(),
                )?;
                Ok(())
            })
        })
        .build()
}
