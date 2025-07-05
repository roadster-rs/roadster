use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::worker::Worker;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(bon::Builder, Serialize, Deserialize, Debug)]
pub struct ExamplePeriodicWorkerArgs {
    a: u64,
}

#[derive(Default)]
pub struct ExamplePeriodicWorker;

#[async_trait]
impl Worker<AppContext, ExamplePeriodicWorkerArgs> for ExamplePeriodicWorker {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::backend::pg::enqueue::PgEnqueuer;

    #[instrument(skip_all)]
    async fn handle(
        &self,
        _state: &AppContext,
        args: ExamplePeriodicWorkerArgs,
    ) -> Result<(), Self::Error> {
        info!("Processing job with args: {args:?}");
        Ok(())
    }
}
