use crate::app::context::AppContext;
use crate::worker::Worker;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::time::Duration;

#[async_trait]
pub trait Enqueuer {
    type Error: std::error::Error;

    async fn enqueue<W, S, Args, ArgsRef, E>(state: &S, args: ArgsRef) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: 'static + Send + Sync + Clone,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // ArgsRef allows the method to take either an owned or borrowed value
        ArgsRef: Send + Sync + Borrow<Args> + Serialize;

    async fn enqueue_delayed<W, S, Args, ArgsRef, E>(
        state: &S,
        args: ArgsRef,
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: 'static + Send + Sync + Clone,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize;

    async fn enqueue_batch<W, S, Args, ArgsRef, E>(
        state: &S,
        args: &[ArgsRef],
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: 'static + Send + Sync + Clone,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize;

    async fn enqueue_batch_delayed<W, S, Args, ArgsRef, E>(
        state: &S,
        args: &[ArgsRef],
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: 'static + Send + Sync + Clone,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize;
}

#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
pub(crate) fn queue_from_worker<W, S, Args, E>(
    state: &S,
) -> Result<String, crate::error::worker::EnqueueError>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let context = AppContext::from_ref(state);
    let worker_enqueue_config = W::enqueue_config(state);
    let enqueue_config = &context.config().service.worker.enqueue_config;

    let queue = if let Some(queue) = worker_enqueue_config.queue {
        queue
    } else if let Some(queue) = enqueue_config.queue.as_ref() {
        queue.to_owned()
    } else {
        let worker_name = W::name();
        tracing::error!(
            worker.name = worker_name,
            "Unable to enqueue job, no queue configured"
        );
        return Err(crate::error::worker::EnqueueError::NoQueue(worker_name));
    };

    Ok(queue)
}

/// Helper function to prepare a job to be enqueued and then enqueue it using the provided `enqueue_fn`.
#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
pub(crate) async fn enqueue<W, S, Args, ArgsRef, E, F>(
    state: &S,
    args: ArgsRef,
    enqueue_fn: F,
) -> crate::error::RoadsterResult<()>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    F: AsyncFn(&S, &str, crate::worker::job::Job) -> crate::error::RoadsterResult<()>,
{
    let worker_name = W::name();

    let args = serde_json::to_value(&args).map_err(crate::error::worker::EnqueueError::Serde)?;
    let job = crate::worker::job::Job::builder()
        .metadata(
            crate::worker::job::JobMetadata::builder()
                .worker_name(worker_name)
                .build(),
        )
        .args(args)
        .build();

    let queue = queue_from_worker::<W, S, Args, E>(state)?;

    enqueue_fn(state, &queue, job).await?;

    Ok(())
}

/// Helper function to prepare a batch of jobs to be enqueued and then enqueue them using the
/// provided `enqueue_fn`.
#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
pub(crate) async fn enqueue_batch<W, S, Args, ArgsRef, E, F>(
    state: &S,
    args: &[ArgsRef],
    enqueue_fn: F,
) -> crate::error::RoadsterResult<()>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    F: AsyncFn(&S, &str, Vec<crate::worker::job::Job>) -> crate::error::RoadsterResult<()>,
{
    let worker_name = W::name();

    let mut args_serialized: Vec<serde_json::Value> = Vec::with_capacity(args.len());
    for arg in args.iter() {
        args_serialized
            .push(serde_json::to_value(arg).map_err(crate::error::worker::EnqueueError::Serde)?);
    }
    let jobs: Vec<crate::worker::job::Job> = args_serialized
        .into_iter()
        .map(|arg| {
            crate::worker::job::Job::builder()
                .metadata(
                    crate::worker::job::JobMetadata::builder()
                        // Todo: We could optimize away this clone by borrowing instead of cloning
                        //  but this would probably require having separate Job/JobMetadata structs
                        //  for enqueue vs dequeue, because the type used for dequeuing needs to
                        //  impl DeserializeOwned.
                        .worker_name(worker_name.clone())
                        .build(),
                )
                .args(arg)
                .build()
        })
        .collect();

    let queue = queue_from_worker::<W, S, Args, E>(state)?;

    enqueue_fn(state, &queue, jobs).await?;

    Ok(())
}

#[cfg(test)]
pub(crate) mod test {
    use crate::app::context::AppContext;
    use crate::worker::Worker;
    use crate::worker::enqueue::Enqueuer;
    use async_trait::async_trait;
    use axum_core::extract::FromRef;
    use serde::{Deserialize, Serialize};
    use std::borrow::Borrow;
    use std::time::Duration;

    pub(crate) struct TestEnqueuer;
    #[async_trait]
    impl Enqueuer for TestEnqueuer {
        type Error = crate::error::Error;

        async fn enqueue<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: ArgsRef,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: 'static + Send + Sync + Clone,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }

        async fn enqueue_delayed<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: ArgsRef,
            _delay: Duration,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: 'static + Send + Sync + Clone,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }

        async fn enqueue_batch<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: &[ArgsRef],
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: 'static + Send + Sync + Clone,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }

        async fn enqueue_batch_delayed<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: &[ArgsRef],
            _delay: Duration,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: 'static + Send + Sync + Clone,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }
    }
}
