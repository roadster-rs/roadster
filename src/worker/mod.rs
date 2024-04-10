pub mod app_worker;
pub mod registry;

use crate::app::App;

use crate::worker::app_worker::AppWorkerConfig;
use app_worker::AppWorker;
use async_trait::async_trait;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::Serialize;

use sidekiq::{RedisPool, Worker, WorkerOpts};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, instrument};

lazy_static! {
    pub static ref DEFAULT_QUEUE_NAMES: Vec<String> =
        ["default"].iter().map(|s| s.to_string()).collect();
}

pub fn queue_names(custom_queue_names: &Vec<String>) -> Vec<String> {
    DEFAULT_QUEUE_NAMES
        .iter()
        .chain(custom_queue_names)
        .unique()
        .map(|s| s.to_owned())
        .collect()
}

/// Worker used by Roadster to wrap the consuming app's workers to add additional behavior. For
/// example, [RoadsterWorker] is by default configured to automatically abort the app's worker
/// when it exceeds a certain timeout.
pub(crate) struct RoadsterWorker<A, Args, W>
where
    A: App,
    Args: Send + Sync + Serialize + 'static,
    W: AppWorker<A, Args>,
{
    inner: W,
    inner_config: AppWorkerConfig,
    _args: PhantomData<Args>,
    _app: PhantomData<A>,
}

impl<A, Args, W> RoadsterWorker<A, Args, W>
where
    A: App,
    Args: Send + Sync + Serialize,
    W: AppWorker<A, Args>,
{
    pub(crate) fn new(inner: W, state: Arc<A::State>) -> Self {
        let config = inner.config(&state);
        Self {
            inner,
            inner_config: config,
            _args: PhantomData,
            _app: PhantomData,
        }
    }
}

#[async_trait]
impl<A, Args, W> Worker<Args> for RoadsterWorker<A, Args, W>
where
    A: App,
    Args: Send + Sync + Serialize,
    W: AppWorker<A, Args>,
{
    fn disable_argument_coercion(&self) -> bool {
        self.inner_config.disable_argument_coercion
    }

    fn opts() -> WorkerOpts<Args, Self>
    where
        Self: Sized,
    {
        let opts = WorkerOpts::new();
        let opts = W::queue()
            .into_iter()
            .fold(opts, |opts, queue| opts.queue(queue));
        let opts = W::retry()
            .into_iter()
            .fold(opts, |opts, retry| opts.retry(retry));
        W::unique_for()
            .into_iter()
            .fold(opts, |opts, unique_for| opts.unique_for(unique_for))
    }

    fn max_retries(&self) -> usize {
        self.inner_config.max_retries
    }

    fn class_name() -> String
    where
        Self: Sized,
    {
        W::class_name()
    }

    async fn perform_async(redis: &RedisPool, args: Args) -> sidekiq::Result<()>
    where
        Self: Sized,
        Args: Send + Sync + Serialize + 'static,
    {
        W::perform_async(redis, args).await
    }

    async fn perform_in(redis: &RedisPool, duration: Duration, args: Args) -> sidekiq::Result<()>
    where
        Self: Sized,
        Args: Send + Sync + Serialize + 'static,
    {
        W::perform_in(redis, duration, args).await
    }

    #[instrument(skip_all)]
    async fn perform(&self, args: Args) -> sidekiq::Result<()> {
        let inner = self.inner.perform(args);

        if let Some(timeout) = self.inner_config.max_duration {
            tokio::time::timeout(timeout, inner).await.map_err(|err| {
                error!(
                    "Worker {} timed out after {} seconds",
                    W::class_name(),
                    timeout.as_secs()
                );
                sidekiq::Error::Any(Box::new(err))
            })?
        } else {
            inner.await
        }
    }
}
