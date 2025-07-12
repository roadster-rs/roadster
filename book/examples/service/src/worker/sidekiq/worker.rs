use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::worker::Worker;
use roadster::worker::config::{RetryConfig, WorkerConfig};
use std::time::Duration;
use tracing::info;

pub struct ExampleWorker;

// Implement the `Worker` trait
#[async_trait]
impl Worker<AppContext, String> for ExampleWorker {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::SidekiqEnqueuer;

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
