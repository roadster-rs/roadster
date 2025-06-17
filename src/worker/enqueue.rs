use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::error::worker::EnqueueError;
use crate::worker::Worker;
use crate::worker::job::{Job, JobMetadata};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
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

pub(crate) fn queue_from_config<W, S, Args, E>(state: &S) -> Result<Cow<str>, EnqueueError>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let context = AppContext::from_ref(state);
    let worker_enqueue_config = W::enqueue_config(state);
    let enqueue_config = &context.config().service.worker.enqueue_config;

    let queue = if let Some(queue) = worker_enqueue_config.queue {
        Cow::from(queue)
    } else if let Some(queue) = enqueue_config.queue.as_ref() {
        Cow::from(queue)
    } else {
        let worker_name = W::name();
        error!(worker_name, "Unable to enqueue job, no queue configured");
        return Err(EnqueueError::NoQueue(worker_name).into());
    };

    Ok(queue)
}

/// Helper function to prepare a job to be enqueued and then enqueue it using the provided `enqueue_fn`.
pub(crate) async fn enqueue<W, S, Args, E, F>(
    state: &S,
    args: &Args,
    enqueue_fn: F,
) -> RoadsterResult<()>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    F: for<'a> AsyncFn(&S, &str, &Job<'a>) -> RoadsterResult<()>,
{
    let worker_name = W::name();

    let args = serde_json::to_string(&args)?;
    let job = Job::builder()
        .metadata(JobMetadata::builder().worker_name(&worker_name).build())
        .args(Cow::from(&args))
        .build();

    let queue = queue_from_config::<W, S, Args, E>(state)?;

    enqueue_fn(state, &queue, &job).await?;

    Ok(())
}

/// Helper function to prepare a batch of jobs to be enqueued and then enqueue them using the
/// provided `enqueue_fn`.
pub(crate) async fn enqueue_batch<W, S, Args, E, F>(
    state: &S,
    args: &[Args],
    enqueue_fn: F,
) -> RoadsterResult<()>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    F: for<'a> AsyncFn(&S, &str, &[Job<'a>]) -> RoadsterResult<()>,
{
    let worker_name = W::name();

    let mut arg_strs: Vec<String> = Vec::with_capacity(args.len());
    for arg in args.iter() {
        arg_strs.push(serde_json::to_string(arg)?);
    }
    let jobs = arg_strs
        .iter()
        .map(|arg| {
            Job::builder()
                .metadata(JobMetadata::builder().worker_name(&worker_name).build())
                .args(Cow::from(arg))
                .build()
        })
        .collect_vec();

    let queue = queue_from_config::<W, S, Args, E>(state)?;

    enqueue_fn(state, &queue, &jobs).await?;

    Ok(())
}
