use crate::app::context::AppContext;
use crate::service::worker::sidekiq::app_worker::AppWorker;
use crate::service::worker::sidekiq::app_worker::AppWorkerConfig;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::Serialize;
use sidekiq::{RedisPool, Worker, WorkerOpts};
use std::marker::PhantomData;
use std::time::Duration;
use tracing::{error, instrument};

/// Worker used by Roadster to wrap the consuming app's workers to add additional behavior. For
/// example, [RoadsterWorker] is by default configured to automatically abort the app's worker
/// when it exceeds a certain timeout.
pub struct RoadsterWorker<S, Args, W>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + 'static,
    W: AppWorker<S, Args>,
{
    inner: W,
    inner_config: AppWorkerConfig,
    _state: PhantomData<S>,
    _args: PhantomData<Args>,
}

impl<S, Args, W> RoadsterWorker<S, Args, W>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize,
    W: AppWorker<S, Args>,
{
    pub(crate) fn new(inner: W, state: &S) -> Self {
        let config = inner.config(state);
        Self {
            inner,
            inner_config: config,
            _state: PhantomData,
            _args: PhantomData,
        }
    }
}

#[async_trait]
impl<S, Args, W> Worker<Args> for RoadsterWorker<S, Args, W>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize,
    W: AppWorker<S, Args>,
{
    fn disable_argument_coercion(&self) -> bool {
        self.inner_config.disable_argument_coercion
    }

    fn opts() -> WorkerOpts<Args, Self>
    where
        Self: Sized,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly,
        // and this method is only used when enqueuing. Instead, Sidekiq.rs will use the
        // `W::opts` implementation directly.
        unimplemented!()
    }

    fn max_retries(&self) -> usize {
        self.inner_config.max_retries
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

    async fn perform_async(_redis: &RedisPool, _args: Args) -> sidekiq::Result<()>
    where
        Self: Sized,
        Args: Send + Sync + Serialize + 'static,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly.
        unimplemented!()
    }

    async fn perform_in(_redis: &RedisPool, _duration: Duration, _args: Args) -> sidekiq::Result<()>
    where
        Self: Sized,
        Args: Send + Sync + Serialize + 'static,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly.
        unimplemented!()
    }

    #[instrument(skip_all)]
    async fn perform(&self, args: Args) -> sidekiq::Result<()> {
        let inner = self.inner.perform(args);

        if self.inner_config.timeout {
            tokio::time::timeout(self.inner_config.max_duration, inner)
                .await
                .map_err(|err| {
                    error!(
                        worker = %W::class_name(),
                        max_duration = %self.inner_config.max_duration.as_secs(),
                        %err,
                        "Worker timed out"
                    );
                    sidekiq::Error::Any(Box::new(err))
                })?
        } else {
            inner.await
        }
    }
}
