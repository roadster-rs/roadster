use crate::app::context::AppContext;
use crate::worker;
use crate::worker::backend::pg::PgmqBackend;
use crate::worker::enqueuer::enqueue_config;
use crate::worker::job::{Job, JobMetadata};
use crate::worker::{Enqueuer, Worker};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::time::Duration;
use tracing::{debug, instrument};

#[async_trait]
impl Enqueuer for PgmqBackend {
    type Error = crate::error::Error;

    #[instrument(skip_all)]
    async fn enqueue<W, S, Args, E>(state: &S, args: &Args) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        let enqueue_config = enqueue_config::<W, S, Args, E>(state)?;
        let worker_name = W::name();
        let context = AppContext::from_ref(state);

        let args = serde_json::to_string(&args)?;
        let job = Job::builder()
            .metadata(JobMetadata::builder().worker_name(&worker_name).build())
            .args(Cow::from(&args))
            .build();
        let id = context.pgmq().send(&enqueue_config.queue, &job).await?;
        debug!(id, "Job enqueued");

        Ok(())
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
        let enqueue_config = enqueue_config::<W, S, Args, E>(state)?;
        let worker_name = W::name();
        let context = AppContext::from_ref(state);

        let args = serde_json::to_string(&args)?;
        let job = Job::builder()
            .metadata(JobMetadata::builder().worker_name(&worker_name).build())
            .args(Cow::from(&args))
            .build();
        let id = context
            .pgmq()
            .send_delay(&enqueue_config.queue, &job, delay.as_secs())
            .await?;
        debug!(id, delay = delay.as_secs(), "Job enqueued");

        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch<W, S, Args, E>(state: &S, args: &[Args]) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        let enqueue_config = enqueue_config::<W, S, Args, E>(state)?;
        let worker_name = W::name();
        let context = AppContext::from_ref(state);

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
        let ids = context
            .pgmq()
            .send_batch(&enqueue_config.queue, &jobs)
            .await?;
        debug!(count = ids.len(), "Jobs enqueued");
        ids.iter().for_each(|id| debug!(id, "Job enqueued"));

        Ok(())
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
        let enqueue_config = enqueue_config::<W, S, Args, E>(state)?;
        let worker_name = W::name();
        let context = AppContext::from_ref(state);

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
        let ids = context
            .pgmq()
            .send_batch_delay(&enqueue_config.queue, &jobs, delay.as_secs())
            .await?;
        debug!(count = ids.len(), "Jobs enqueued");
        ids.iter().for_each(|id| debug!(id, "Job enqueued"));

        Ok(())
    }
}
