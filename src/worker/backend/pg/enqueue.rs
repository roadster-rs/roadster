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

pub struct PgEnqueuer;

#[async_trait]
impl Enqueuer for PgEnqueuer {
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
                let msg_id = context.pgmq().send(queue, &job).await?;
                debug!(
                    job.id = %job.metadata.id,
                    job.msg_id = msg_id,
                    "Job enqueued"
                );
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
                let msg_id = context.pgmq().send_delay(queue, &job, delay).await?;
                debug!(
                    job.id = %job.metadata.id,
                    job.msg_id = msg_id,
                    job.delay = delay.as_secs(),
                    "Job enqueued"
                );
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
                // Todo: Restore enqueuing batch with a single DB call
                for job in jobs.iter() {
                    let msg_id = context.pgmq().send(queue, &job).await?;
                    debug!(
                        job.id = %job.metadata.id,
                        job.msg_id = msg_id,
                        "Job enqueued"
                    )
                }
                debug!(count = jobs.len(), "Jobs enqueued");
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
                // Todo: Restore enqueuing batch with a single DB call
                for job in jobs.iter() {
                    let msg_id = context.pgmq().send_delay(queue, &job, delay).await?;
                    debug!(
                        job.id = %job.metadata.id,
                        job.msg_id = msg_id,
                        job.delay = delay.as_secs(),
                        "Job enqueued"
                    )
                }
                debug!(count = jobs.len(), delay = delay.as_secs(), "Jobs enqueued");
                Ok(())
            },
        )
        .await
    }
}
