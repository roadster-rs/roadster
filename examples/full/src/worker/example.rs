use crate::app_state::AppState;
use async_trait::async_trait;
use roadster::worker::Worker;
use tracing::{info, instrument};

pub struct ExampleWorker;

#[async_trait]
impl Worker<AppState, String> for ExampleWorker {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::SidekiqEnqueuer;

    #[instrument(skip_all)]
    async fn handle(&self, _state: &AppState, args: String) -> Result<(), Self::Error> {
        info!("Processing job with args: {args}");
        Ok(())
    }
}
