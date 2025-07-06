use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::Worker;
use crate::worker::backend::sidekiq::processor::{
    SidekiqProcessor, SidekiqProcessorError, SidekiqProcessorInner,
};
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[non_exhaustive]
pub struct SidekiqProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) inner: SidekiqProcessorInner<S>,
}

impl<S> SidekiqProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            inner: SidekiqProcessorInner {
                state: state.clone(),
                queues: Default::default(),
                processor: Default::default(),
            },
        }
    }

    pub fn build(self) -> SidekiqProcessor<S> {
        SidekiqProcessor::new(self.inner)
    }

    pub fn with_processor(mut self, processor: ::sidekiq::Processor) -> Self {
        self.inner.processor = Some(processor);
        self
    }

    pub fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering Sidekiq worker");

        self.register_internal(worker, name, false)?;

        Ok(self)
    }

    fn register_internal<W, Args, E>(
        &mut self,
        worker: W,
        name: String,
        skip_duplicate: bool,
    ) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let context = AppContext::from_ref(&self.inner.state);
        let enqueue_config = &context.config().service.worker.enqueue_config;
        let worker_enqueue_config = W::enqueue_config(&self.inner.state);

        let queue = worker_enqueue_config
            .queue
            .as_ref()
            .or(enqueue_config.queue.as_ref());
        let queue = if let Some(queue) = queue {
            queue
        } else {
            error!(
                worker_name = W::name(),
                "Unable to register worker, no queue configured"
            );
            return Err(SidekiqProcessorError::NoQueue(W::name()).into());
        };
        self.inner.queues.insert(queue.to_owned());

        // todo
        // if self
        //     .inner
        //     .workers
        //     .insert(
        //         name.clone(),
        //         WorkerWrapper::new(&self.inner.state, worker, worker_enqueue_config)?,
        //     )
        //     .is_some()
        //     && !skip_duplicate
        // {
        //     return Err(SidekiqProcessorError::AlreadyRegistered(name).into());
        // }

        Ok(())
    }
}
