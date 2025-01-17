use crate::app::context::AppContext;
use crate::service::worker::sidekiq::app_worker::AppWorkerConfig;
use crate::service::worker::sidekiq::app_worker::DEFAULT_MAX_DURATION;
use async_trait::async_trait;
use serde::Serialize;
use sidekiq::{RedisPool, Worker, WorkerOpts};
use std::marker::PhantomData;
use std::time::Duration;
use tracing::{error, instrument};

/// Worker used by Roadster to wrap the consuming app's workers to add additional behavior. For
/// example, [RoadsterWorker] is by default configured to automatically abort the app's worker
/// when it exceeds a certain timeout.
pub(crate) struct RoadsterWorker<Args, W>
where
    Args: Send + Sync + Serialize + 'static,
    W: Worker<Args>,
{
    inner: W,
    inner_config: Option<AppWorkerConfig>,
    context: AppContext,
    _args: PhantomData<Args>,
}

impl<Args, W> RoadsterWorker<Args, W>
where
    Args: Send + Sync + Serialize,
    W: Worker<Args>,
{
    pub(crate) fn new(context: &AppContext, inner: W, config: Option<AppWorkerConfig>) -> Self {
        Self {
            inner,
            inner_config: config,
            context: context.clone(),
            _args: PhantomData,
        }
    }
}

#[async_trait]
impl<Args, W> Worker<Args> for RoadsterWorker<Args, W>
where
    Args: Send + Sync + Serialize,
    W: Worker<Args>,
{
    fn disable_argument_coercion(&self) -> bool {
        self.inner_config
            .as_ref()
            .and_then(|config| config.disable_argument_coercion)
            .unwrap_or_else(|| {
                self.context
                    .config()
                    .service
                    .sidekiq
                    .custom
                    .app_worker
                    .disable_argument_coercion
                    .unwrap_or_else(|| W::disable_argument_coercion(&self.inner))
            })
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
        self.inner_config
            .as_ref()
            .and_then(|config| config.max_retries)
            .unwrap_or_else(|| {
                self.context
                    .config()
                    .service
                    .sidekiq
                    .custom
                    .app_worker
                    .max_retries
                    .unwrap_or_else(|| W::max_retries(&self.inner))
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

        let timeout = self
            .inner_config
            .as_ref()
            .and_then(|config| config.timeout)
            .unwrap_or_else(|| {
                self.context
                    .config()
                    .service
                    .sidekiq
                    .custom
                    .app_worker
                    .timeout
                    .unwrap_or_default()
            });

        if timeout {
            let max_duration = self
                .inner_config
                .as_ref()
                .and_then(|config| config.max_duration)
                .unwrap_or_else(|| {
                    self.context
                        .config()
                        .service
                        .sidekiq
                        .custom
                        .app_worker
                        .max_duration
                        .unwrap_or(DEFAULT_MAX_DURATION)
                });

            tokio::time::timeout(max_duration, inner)
                .await
                .map_err(|err| {
                    error!(
                        worker = %W::class_name(),
                        max_duration = %max_duration.as_secs(),
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
