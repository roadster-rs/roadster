use crate::latch::Countdown;
use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::worker::config::EnqueueConfig;
use roadster::worker::{PgEnqueuer, SidekiqEnqueuer, Worker};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub const QUEUE: &'static str = "worker_bench";

#[derive(bon::Builder, Serialize, Deserialize, Debug)]
pub struct ExampleWorkerArgs {
    #[builder(into)]
    foo: String,
    bar: i32,
}

#[derive(bon::Builder)]
pub struct PgExampleWorker {
    latch: Countdown,
}

#[derive(bon::Builder)]
pub struct SidekiqExampleWorker {
    latch: Countdown,
}

#[async_trait]
impl Worker<AppContext, ExampleWorkerArgs> for PgExampleWorker {
    type Error = roadster::error::Error;
    type Enqueuer = PgEnqueuer;

    fn enqueue_config(_state: &AppContext) -> EnqueueConfig {
        EnqueueConfig::builder().queue(QUEUE).build()
    }

    #[instrument(skip_all)]
    async fn handle(
        &self,
        _state: &AppContext,
        _args: ExampleWorkerArgs,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn on_complete(&self) {
        self.latch.count_down();
    }
}

#[async_trait]
impl Worker<AppContext, ExampleWorkerArgs> for SidekiqExampleWorker {
    type Error = roadster::error::Error;
    type Enqueuer = SidekiqEnqueuer;

    fn enqueue_config(_state: &AppContext) -> EnqueueConfig {
        EnqueueConfig::builder().queue(QUEUE).build()
    }

    #[instrument(skip_all)]
    async fn handle(
        &self,
        _state: &AppContext,
        _args: ExampleWorkerArgs,
    ) -> Result<(), Self::Error> {
        self.latch.count_down();
        Ok(())
    }
}
