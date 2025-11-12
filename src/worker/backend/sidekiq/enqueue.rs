use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::job::Job;
use crate::worker::{Enqueuer, Worker, enqueue};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::time::Duration;
use tracing::{debug, instrument};

pub struct SidekiqEnqueuer;

#[async_trait]
impl Enqueuer for SidekiqEnqueuer {
    type Error = crate::error::Error;

    #[instrument(skip_all)]
    async fn enqueue<W, S, Args, ArgsRef, E>(state: &S, args: ArgsRef) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: 'static + Send + Sync + Clone,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue::<W, _, _, _, _, _>(
            state,
            args,
            async |state, queue, job: Job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job id?
                ::sidekiq::perform_async(
                    context.redis_enqueue(),
                    job.metadata.worker_name.to_owned(),
                    queue.to_owned(),
                    &job,
                )
                .await?;
                debug!(job.id = %job.metadata.id, "Job enqueued");
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
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
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue::<W, _, _, _, _, _>(
            state,
            args,
            async move |state, queue, job: Job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job id?
                ::sidekiq::perform_in(
                    context.redis_enqueue(),
                    delay,
                    job.metadata.worker_name.to_owned(),
                    queue.to_owned(),
                    &job,
                )
                .await?;
                debug!(job.id = %job.metadata.id, job.delay = delay.as_secs(), "Job enqueued");
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch<W, S, Args, ArgsRef, E>(
        state: &S,
        args: &[ArgsRef],
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: 'static + Send + Sync + Clone,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue_batch::<W, _, _, _, _, _>(
            state,
            args,
            async |state, queue, jobs: Vec<Job>| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job ids?
                // Todo: update `sidekiq` to batch enqueue?
                for job in jobs {
                    ::sidekiq::perform_async(
                        context.redis_enqueue(),
                        job.metadata.worker_name.to_owned(),
                        queue.to_owned(),
                        &job,
                    )
                    .await?;
                    debug!(job.id = %job.metadata.id, "Job enqueued");
                }
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
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
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue_batch::<W, _, _, _, _, _>(
            state,
            args,
            async move |state, queue, jobs: Vec<Job>| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job ids?
                // Todo: update `sidekiq` to batch enqueue?
                for job in jobs {
                    ::sidekiq::perform_in(
                        context.redis_enqueue(),
                        delay,
                        job.metadata.worker_name.to_owned(),
                        queue.to_owned(),
                        &job,
                    )
                    .await?;
                    debug!(job.id = %job.metadata.id, job.delay = delay.as_secs(), "Job enqueued");
                }
                Ok(())
            },
        )
        .await
    }
}
