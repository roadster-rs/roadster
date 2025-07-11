mod worker_with_configs;

use async_trait::async_trait;
use axum::extract::State;
use roadster::app::RoadsterApp;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::service::worker::backend::sidekiq::SidekiqWorkerService;
use roadster::service::worker::sidekiq::app_worker::{AppWorker, AppWorkerConfig};
use sidekiq::Worker;
use std::time::Duration;
use tracing::info;

pub struct ExampleWorker {
    // If the worker needs access to your app's state, it can be added as a field in the worker.
    state: AppContext,
}

impl ExampleWorker {
    pub fn new(state: &AppContext) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

// Implement the `Worker` trait
#[async_trait]
impl Worker<String> for ExampleWorker {
    async fn perform(&self, args: String) -> sidekiq::Result<()> {
        info!("Processing job with args: {args}");
        Ok(())
    }
}

fn build_app() -> RoadsterApp<AppContext> {
    RoadsterApp::builder()
        // Use the default `AppContext` for this example
        .state_provider(|context| Ok(context))
        // Register the Sidekiq worker service
        .add_service_provider(move |registry, state| {
            Box::pin(async move {
                registry
                    .register_builder(
                        SidekiqWorkerService::builder(state)
                            .await?
                            // Register the `ExampleWorker` with the sidekiq service
                            .register_worker(ExampleWorker::new(state))?
                            // Optionally register the worker with worker-level config overrides
                            .register_worker_with_config(
                                ExampleWorker::new(state),
                                AppWorkerConfig::builder()
                                    .max_retries(3)
                                    .timeout(true)
                                    .max_duration(Duration::from_secs(30))
                                    .build(),
                            )?
                            // Register the `ExampleWorker` to run as a periodic cron job
                            .register_periodic_worker(
                                sidekiq::periodic::builder("* * * * * *")?
                                    .name("Example periodic worker"),
                                ExampleWorker::new(state),
                                "Periodic example args".to_string(),
                            )
                            .await?,
                    )
                    .await?;
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
