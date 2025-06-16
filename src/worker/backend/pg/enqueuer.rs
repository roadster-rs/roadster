use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker;
use crate::worker::backend::pg::PgBackend;
use crate::worker::enqueue::enqueue_config;
use crate::worker::job::{Job, JobMetadata};
use crate::worker::{Enqueuer, Worker, enqueue};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use itertools::Itertools;
use pgmq::PGMQueue;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;
use tracing::{debug, instrument};

#[async_trait]
impl Enqueuer for PgBackend {
    type Error = crate::error::Error;

    #[instrument(skip_all)]
    async fn enqueue<W, S, Args, E>(state: &S, args: &Args) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        enqueue::enqueue::<W, _, _, _, _>(
            state,
            args,
            async |state, queue, job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let id = context.pgmq().send(queue, job).await?;
                debug!(id, "Job enqueued");
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_delayed<W, S, Args, E>(
        state: &S,
        args: &Args,
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        enqueue::enqueue::<W, _, _, _, _>(
            state,
            args,
            async move |state, queue, job| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let id = context
                    .pgmq()
                    .send_delay(queue, job, delay.as_secs())
                    .await?;
                debug!(id, delay = delay.as_secs(), "Job enqueued");
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch<W, S, Args, E>(state: &S, args: &[Args]) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        enqueue::enqueue_batch::<W, _, _, _, _>(
            state,
            args,
            async |state, queue, jobs| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let ids = context.pgmq().send_batch(queue, jobs).await?;
                debug!(count = ids.len(), "Jobs enqueued");
                ids.iter().for_each(|id| debug!(id, "Job enqueued"));
                Ok(())
            },
        )
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch_delayed<W, S, Args, E>(
        state: &S,
        args: &[Args],
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        enqueue::enqueue_batch::<W, _, _, _, _>(
            state,
            args,
            async move |state, queue, jobs| -> RoadsterResult<()> {
                let context = AppContext::from_ref(state);
                let ids = context
                    .pgmq()
                    .send_batch_delay(queue, jobs, delay.as_secs())
                    .await?;
                debug!(count = ids.len(), delay = delay.as_secs(), "Jobs enqueued");
                ids.iter()
                    .for_each(|id| debug!(id, delay = delay.as_secs(), "Job enqueued"));
                Ok(())
            },
        )
        .await
    }
}
