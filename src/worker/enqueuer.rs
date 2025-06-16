use crate::app::context::AppContext;
use crate::error::worker::EnqueueError;
use crate::worker::job::JobMetadata;
use crate::worker::{QueueBackend, Worker};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, instrument};
use typed_builder::TypedBuilder;

#[async_trait]
pub trait Enqueuer {
    type Error: std::error::Error;

    async fn enqueue<W, S, Args, E>(state: &S, args: &Args) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>;

    async fn enqueue_delayed<W, S, Args, E>(
        state: &S,
        args: &Args,
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>;

    async fn enqueue_batch<W, S, Args, E>(state: &S, args: &[Args]) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>;

    async fn enqueue_batch_delayed<W, S, Args, E>(
        state: &S,
        args: &[Args],
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>;
}

/// Same as [`crate::worker::EnqueueConfig`], except that all the required fields are not [`Option`].
#[derive(Debug, TypedBuilder)]
#[non_exhaustive]
struct EnqueueConfigRequired {
    pub queue: String,
    // Todo: this might not be needed depending on how we end up liking the Enqueue trait.
    pub backend: QueueBackend,
}

pub(crate) fn enqueue_config<W, S, Args, E>(
    state: &S,
) -> Result<EnqueueConfigRequired, EnqueueError>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let context = AppContext::from_ref(state);
    let worker_enqueue_config = W::enqueue_config(state);
    let enqueue_config = &context.config().service.worker.enqueue_config;

    let backend = if let Some(backend) = worker_enqueue_config.backend {
        backend
    } else if let Some(backend) = enqueue_config.backend.as_ref() {
        backend.to_owned()
    } else {
        let worker_name = W::name();
        error!(worker_name, "Unable to enqueue job, no backend configured");
        return Err(EnqueueError::NoBackend(worker_name).into());
    };

    let queue = if let Some(queue) = worker_enqueue_config.queue {
        queue
    } else if let Some(queue) = enqueue_config.queue.as_ref() {
        queue.to_owned()
    } else {
        let worker_name = W::name();
        error!(worker_name, "Unable to enqueue job, no queue configured");
        return Err(EnqueueError::NoQueue(worker_name).into());
    };

    Ok(EnqueueConfigRequired::builder()
        .backend(backend)
        .queue(queue)
        .build())
}
