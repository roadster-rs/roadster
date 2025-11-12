use crate::app::context::AppContext;
use crate::worker::job::Job;
use crate::worker::{Worker, WorkerWrapper};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use sidekiq::{RedisPool, WorkerOpts};
use std::marker::PhantomData;
use std::time::Duration;

/// [`::sidekiq::Worker`] used by Roadster to pass a [`Worker`] to sidekiq.
#[derive(Clone)]
pub(crate) struct RoadsterWorker<S, W, Args, E>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    W: 'static + Worker<S, Args, Error = E>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    E: 'static + Send + Sync + std::error::Error,
{
    state: S,
    inner: WorkerWrapper<S>,
    _args: PhantomData<Args>,
    _worker: PhantomData<W>,
    _error: PhantomData<E>,
}

impl<S, W, Args, E> RoadsterWorker<S, W, Args, E>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    W: 'static + Worker<S, Args, Error = E>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    E: 'static + Send + Sync + std::error::Error,
{
    pub(crate) fn new(state: &S, inner: WorkerWrapper<S>) -> Self {
        Self {
            state: state.clone(),
            inner,
            _args: Default::default(),
            _worker: Default::default(),
            _error: Default::default(),
        }
    }
}

#[async_trait]
impl<S, W, Args, E> ::sidekiq::Worker<Job> for RoadsterWorker<S, W, Args, E>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    W: 'static + Worker<S, Args, Error = E>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    E: 'static + Send + Sync + std::error::Error,
{
    fn disable_argument_coercion(&self) -> bool {
        let context = AppContext::from_ref(&self.state);
        self.inner
            .inner
            .worker_config
            .sidekiq
            .as_ref()
            .and_then(|config| config.disable_argument_coercion)
            .unwrap_or_else(|| {
                context
                    .config()
                    .service
                    .worker
                    .worker_config
                    .sidekiq
                    .as_ref()
                    .and_then(|config| config.disable_argument_coercion)
                    .unwrap_or_default()
            })
    }

    fn opts() -> WorkerOpts<Job, Self>
    where
        Self: Sized,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly,
        // and this method is only used when enqueuing. Instead, Sidekiq.rs will use the
        // `W::opts` implementation directly.
        unimplemented!()
    }

    fn max_retries(&self) -> usize {
        let context = AppContext::from_ref(&self.state);
        self.inner
            .inner
            .worker_config
            .retry_config
            .as_ref()
            .and_then(|config| config.max_retries)
            .unwrap_or_else(|| {
                context
                    .config()
                    .service
                    .worker
                    .worker_config
                    .retry_config
                    .as_ref()
                    .and_then(|config| config.max_retries)
                    .unwrap_or_default()
            })
    }

    fn class_name() -> String
    where
        Self: Sized,
    {
        // This method is implemented because it's used both when registering the worker, and
        // when enqueuing a job. We forward the implementation to `W::classname` because that's
        // what Sidekiq.rs uses specifically. If we attempt to override this, our impl will be used
        // when registering the worker, but not when enqueuing a job, so the worker will not pick
        // up the jobs.
        W::name()
    }

    async fn perform_async(_redis: &RedisPool, _args: Job) -> sidekiq::Result<()>
    where
        Self: Sized,
    {
        unimplemented!("`RoadsterWorker` should not be enqueued directly")
    }

    async fn perform_in(_redis: &RedisPool, _duration: Duration, _args: Job) -> sidekiq::Result<()>
    where
        Self: Sized,
    {
        unimplemented!("`RoadsterWorker` should not be enqueued directly")
    }

    async fn perform(&self, job: Job) -> sidekiq::Result<()> {
        self.inner
            .handle(&self.state, &job.metadata, job.args)
            .await
            .map_err(|err| sidekiq::Error::Any(Box::new(err)))?;
        Ok(())
    }
}
