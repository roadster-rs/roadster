use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::worker::Worker;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

#[derive(bon::Builder, Serialize, Deserialize, Debug)]
pub struct ExampleWorkerArgs {
    #[builder(into)]
    foo: String,
    bar: i32,
}

#[derive(Default)]
pub struct ExampleWorker;

#[async_trait]
impl Worker<AppContext, ExampleWorkerArgs> for ExampleWorker {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::backend::pg::enqueue::PgEnqueuer;

    #[instrument(skip_all)]
    async fn handle(
        &self,
        _state: &AppContext,
        args: ExampleWorkerArgs,
    ) -> Result<(), Self::Error> {
        info!("Processing job with args: {args:?}");
        Ok(())
    }
}
