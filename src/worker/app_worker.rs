use crate::app::App;
use crate::app_context::AppContext;
use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use sidekiq::Worker;
use std::sync::Arc;
use std::time::Duration;
use typed_builder::TypedBuilder;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
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

#[async_trait]
pub trait AppWorker<A, Args>: Worker<Args>
where
    Self: Sized,
    A: App,
    Args: Send + Sync + serde::Serialize + 'static,
{
    fn build(state: &A::State) -> Self;

    async fn enqueue(state: &A::State, args: Args) -> anyhow::Result<()> {
        let context: Arc<AppContext> = state.clone().into();
        Self::perform_async(&context.redis, args).await?;
        Ok(())
    }

    fn config(&self, state: &A::State) -> AppWorkerConfig {
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
}
