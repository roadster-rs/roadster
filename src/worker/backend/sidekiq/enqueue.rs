use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::backend::sidekiq::SidekiqBackend;
use crate::worker::{Enqueuer, Worker, enqueue};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::time::Duration;
use tracing::{debug, instrument};

#[async_trait]
impl Enqueuer for SidekiqBackend {
    type Error = crate::error::Error;

    #[instrument(skip_all)]
    async fn enqueue<W, S, Args, ArgsRef, E>(state: &S, args: ArgsRef) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue::<W, _, _, _, _, _>(
            state,
            args,
            async |state, queue, job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job id?
                ::sidekiq::perform_async(
                    context.redis_enqueue(),
                    queue.to_owned(),
                    job.metadata.worker_name.to_owned(),
                    &job.args,
                )
                .await?;
                debug!("Job enqueued");
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
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue::<W, _, _, _, _, _>(
            state,
            args,
            async move |state, queue, job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job id?
                ::sidekiq::perform_in(
                    context.redis_enqueue(),
                    delay,
                    queue.to_owned(),
                    job.metadata.worker_name.to_owned(),
                    &job.args,
                )
                .await?;
                debug!(delay = delay.as_secs(), "Job enqueued");
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
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue_batch::<W, _, _, _, _, _>(
            state,
            args,
            async |state, queue, jobs| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job ids?
                // Todo: update `sidekiq` to batch enqueue?
                for job in jobs {
                    ::sidekiq::perform_async(
                        context.redis_enqueue(),
                        queue.to_owned(),
                        job.metadata.worker_name.to_owned(),
                        &job.args,
                    )
                    .await?;
                    debug!("Job enqueued");
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
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        enqueue::enqueue_batch::<W, _, _, _, _, _>(
            state,
            args,
            async move |state, queue, jobs| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                // Todo: update `sidekiq` to return the job ids?
                // Todo: update `sidekiq` to batch enqueue?
                for job in jobs {
                    ::sidekiq::perform_in(
                        context.redis_enqueue(),
                        delay,
                        queue.to_owned(),
                        job.metadata.worker_name.to_owned(),
                        &job.args,
                    )
                    .await?;
                    debug!(delay = delay.as_secs(), "Job enqueued");
                }
                Ok(())
            },
        )
        .await
    }
}
