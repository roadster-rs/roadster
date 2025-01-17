use async_trait::async_trait;
use sidekiq::Worker;
use tracing::{info, instrument};

#[derive(Default)]
pub struct ExampleWorker;

#[async_trait]
impl Worker<String> for ExampleWorker {
    #[instrument(skip_all)]
    async fn perform(&self, args: String) -> sidekiq::Result<()> {
        info!("Processing job with args: {args}");
        Ok(())
    }
}
