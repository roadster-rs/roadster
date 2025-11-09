//! Background task queue service backed by Redis using [rusty-sidekiq](https://docs.rs/rusty-sidekiq).
//! The [`SidekiqWorkerService`] is a simple wrapper around a [`SidekiqProcessor`].

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::Service;
use crate::worker::backend::sidekiq::processor::SidekiqProcessor;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use tokio_util::sync::CancellationToken;
use tracing::{debug, instrument};

pub(crate) const NAME: &str = "worker-sidekiq";

#[derive(bon::Builder)]
#[non_exhaustive]
pub struct SidekiqWorkerService<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) processor: SidekiqProcessor<S>,
}

#[async_trait]
impl<S> Service<S> for SidekiqWorkerService<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        NAME.to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        enabled(&AppContext::from_ref(state), &self.processor)
    }

    #[instrument(skip_all)]
    async fn before_run(&self, state: &S) -> RoadsterResult<()> {
        self.processor.before_run(state).await?;
        Ok(())
    }

    async fn run(
        self: Box<Self>,
        state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        self.processor.run(state, cancel_token).await;
        Ok(())
    }
}

pub(crate) fn enabled<S>(context: &AppContext, processor: &SidekiqProcessor<S>) -> bool
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let config = &context.config().service.worker.sidekiq;
    if !config.common.enabled(context) {
        debug!("Sidekiq is not enabled in the config.");
        return false;
    }

    let dedicated_workers: u64 = config
        .custom
        .common
        .queue_config
        .values()
        .map(|config| u64::from(config.num_workers.unwrap_or_default()))
        .sum();

    if config.custom.common.num_workers == 0 && dedicated_workers == 0 {
        debug!("Sidekiq configured with 0 worker tasks.");
        return false;
    }

    let queues_empty = if let Some(queues) = config.custom.common.queues.as_ref() {
        queues.is_empty()
    } else {
        processor.inner.queues.is_empty()
    };

    if queues_empty && dedicated_workers == 0 {
        debug!("Sidekiq configured with 0 worker queues.");
        return false;
    }

    if context.redis_fetch().is_none() {
        debug!("No 'redis-fetch' pool connections available.");
        return false;
    }

    true
}
