use crate::app::App;
use crate::app_context::AppContext;
use async_trait::async_trait;
use clap::builder::Str;
use convert_case::{Case, Casing};
use derive_builder::Builder;
use itertools::Itertools;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use sidekiq::{RedisPool, Worker, WorkerOpts};
use std::env::Args;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;
use typed_builder::TypedBuilder;

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

#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
pub struct AppWorkerConfig {
    pub max_retries: usize,
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub timeout: Option<Duration>,
    pub disable_argument_coercion: bool,
}

impl Default for AppWorkerConfig {
    fn default() -> Self {
        AppWorkerConfig::builder()
            .max_retries(5)
            .timeout(Some(Duration::from_secs(60)))
            .disable_argument_coercion(false)
            .build()
    }
}

pub trait AppWorker<A, Args>: Worker<Args>
where
    A: App,
{
    fn config(&self, state: &A::State) -> AppWorkerConfig {
        // AppWorkerConfig::default().queue(self.queue(state))
        AppWorkerConfig::builder()
            .max_retries(AppWorker::max_retries(self, state))
            .timeout(self.timeout(state))
            .disable_argument_coercion(AppWorker::disable_argument_coercion(self, state))
            .build()
    }

    fn queue() -> Option<String> {
        None
    }

    fn retry() -> Option<bool> {
        None
    }

    fn unique_for() -> Option<Duration> {
        None
    }

    fn max_retries(&self, state: &A::State) -> usize {
        let context: Arc<AppContext> = state.clone().into();
        context.config.worker.sidekiq.worker_config.max_retries
    }

    fn timeout(&self, state: &A::State) -> Option<Duration> {
        let context: Arc<AppContext> = state.clone().into();
        context.config.worker.sidekiq.worker_config.timeout
    }

    fn disable_argument_coercion(&self, state: &A::State) -> bool {
        let context: Arc<AppContext> = state.clone().into();
        context
            .config
            .worker
            .sidekiq
            .worker_config
            .disable_argument_coercion
    }

    fn use_fqcn() -> bool {
        true
    }
}

pub struct RoadsterWorker<A, Args, W>
where
    A: App,
    Args: Send + Sync + Serialize,
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
        let self_name = {
            let name = std::any::type_name::<W>().to_case(Case::UpperCamel);
            name.split("::")
                .last()
                .unwrap_or(&name)
                .to_case(Case::UpperCamel)
        };
        let inner_name = if W::use_fqcn() {
            std::any::type_name::<W>().to_case(Case::UpperCamel)
        } else {
            W::class_name()
        };

        format!("{}::{}", self_name, inner_name)
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

    async fn perform(&self, args: Args) -> sidekiq::Result<()> {
        self.inner.perform(args).await
    }
}
