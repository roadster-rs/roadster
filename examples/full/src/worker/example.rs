use crate::app::App;
use crate::app_state::AppState;
use async_trait::async_trait;
use roadster::service::worker::sidekiq::app_worker::AppWorker;
use sidekiq::Worker;
use tracing::{info, instrument};

pub struct ExampleWorker {}

#[async_trait]
impl Worker<String> for ExampleWorker {
    #[instrument(skip_all)]
    async fn perform(&self, args: String) -> sidekiq::Result<()> {
        info!("Processing job with args: {args}");
        Ok(())
    }
}

#[async_trait]
impl AppWorker<App, String> for ExampleWorker {
    fn build(_context: &AppState) -> Self {
        Self {}
    }
}
