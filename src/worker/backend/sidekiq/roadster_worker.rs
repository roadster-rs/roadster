use crate::app::context::AppContext;
use crate::worker::WorkerWrapper;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::Serialize;
use sidekiq::{RedisPool, WorkerOpts};
use std::marker::PhantomData;
use std::time::Duration;

/// [`::sidekiq::Worker`] used by Roadster to pass a [`crate::worker::Worker`] to sidekiq.
pub(crate) struct RoadsterWorker<S, Args, W>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + 'static,
    W: ::sidekiq::Worker<Args>,
{
    state: S,
    inner: WorkerWrapper<S>,
    _args: PhantomData<Args>,
    _worker: PhantomData<W>,
}

impl<S, Args, W> RoadsterWorker<S, Args, W>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + 'static,
    W: ::sidekiq::Worker<Args>,
{
    pub(crate) fn new(state: &S, inner: WorkerWrapper<S>) -> Self {
        Self {
            state: state.clone(),
            inner,
            _args: Default::default(),
            _worker: Default::default(),
        }
    }
}

#[async_trait]
impl<S, Args, W> ::sidekiq::Worker<serde_json::Value> for RoadsterWorker<S, Args, W>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + 'static,
    W: ::sidekiq::Worker<Args>,
{
    fn disable_argument_coercion(&self) -> bool {
        let context = AppContext::from_ref(&self.state);
        self.inner
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

    fn opts() -> WorkerOpts<serde_json::Value, Self>
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
        W::class_name()
    }

    async fn perform_async(_redis: &RedisPool, _args: serde_json::Value) -> sidekiq::Result<()>
    where
        Self: Sized,
    {
        unimplemented!("`RoadsterWorker` should not be enqueued directly")
    }

    async fn perform_in(
        _redis: &RedisPool,
        _duration: Duration,
        _args: serde_json::Value,
    ) -> sidekiq::Result<()>
    where
        Self: Sized,
    {
        unimplemented!("`RoadsterWorker` should not be enqueued directly")
    }

    async fn perform(&self, args: serde_json::Value) -> sidekiq::Result<()> {
        self.inner
            .handle(&self.state, args)
            .await
            .map_err(|err| sidekiq::Error::Any(Box::new(err)))?;
        Ok(())
    }
}
