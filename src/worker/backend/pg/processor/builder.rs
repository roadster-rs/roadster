use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::Worker;
use crate::worker::backend::pg::processor::{
    PgProcessorError, Processor, ProcessorInner, WorkerWrapper,
};
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[non_exhaustive]
pub struct ProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub inner: ProcessorInner<S>,
}

impl<S> ProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            inner: ProcessorInner {
                state: state.clone(),
                queues: Default::default(),
                workers: Default::default(),
            },
        }
    }

    pub fn build(self) -> Processor<S> {
        Processor::new(self.inner)
    }

    pub async fn register<W, Args, E>(&mut self, worker: W) -> RoadsterResult<&mut Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering PG worker");

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
            return Err(PgProcessorError::NoQueue(W::name()).into());
        };
        self.inner.queues.insert(queue.to_owned());

        if self
            .inner
            .workers
            .insert(
                name.clone(),
                WorkerWrapper::new(&self.inner.state, worker, worker_enqueue_config)?,
            )
            .is_some()
        {
            return Err(PgProcessorError::AlreadyRegistered(name).into());
        }
        Ok(self)
    }
}
