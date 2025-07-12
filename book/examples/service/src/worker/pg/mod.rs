use async_trait::async_trait;
use axum::extract::State;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::worker::PgWorkerService;
use roadster::worker::config::{RetryConfig, WorkerConfig};
use roadster::worker::{PeriodicArgs, PgProcessor, Worker};
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

pub struct ExampleWorker;

// Implement the `Worker` trait
#[async_trait]
impl Worker<AppContext, String> for ExampleWorker {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::PgEnqueuer;

    // Optionally provide worker-level config overrides
    fn worker_config(&self, _state: &AppContext) -> WorkerConfig {
        WorkerConfig::builder()
            .retry_config(RetryConfig::builder().max_retries(3).build())
            .timeout(true)
            .max_duration(Duration::from_secs(30))
            .build()
    }

    async fn handle(&self, _state: &AppContext, args: String) -> Result<(), Self::Error> {
        info!("Processing job with args: {args}");
        Ok(())
    }
}

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

async fn example_get(State(state): State<AppContext>) -> RoadsterResult<()> {
    // Enqueue the job in your API handler
    ExampleWorker::enqueue(&state, "Example".to_string()).await?;

    Ok(())
}
