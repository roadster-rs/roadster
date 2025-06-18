use crate::app::context::AppContext;
use crate::config::CustomConfig;
use crate::util::types;
use crate::worker::Processor;
use crate::worker::enqueue::Enqueuer;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::time::Duration;
use tracing::instrument;
use typed_builder::TypedBuilder;
use validator::Validate;

/// Worker configuration options to use when enqueuing a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`Worker::enqueue_config`] method.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct EnqueueConfig {
    /// The name of the queue used to enqueue jobs. Multiple workers can enqueue jobs on the same
    /// queue, which is particularly useful for workers that may not have many jobs. However,
    /// workers can also be configured to use a dedicated queue.
    ///
    /// Note: when used with a Postgres backend with `pgmq`, this will be used in table names.
    /// Postgres generally has a length limit for table names, so care should be taken to ensure
    /// this queue name is not too long or else the queue name will be truncated when used
    /// with `pgmq`.
    #[serde(default)]
    #[builder(default, setter(strip_option(fallback = queue_opt)))]
    pub queue: Option<String>,
}

/// Worker configuration options to use when handling a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`Worker::worker_config`] method.
// Todo: Add success/failure actions? They might by Pgmq-specific...
// Todo: Add custom config for custom worker backends?
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig {
    /// The maximum number of times a job should be retried on failure.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub max_retries: Option<usize>,

    /// True if Roadster should enforce a timeout on the app's workers. The default duration of
    /// the timeout can be configured with the `max-duration` option.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub timeout: Option<bool>,

    /// The maximum duration workers should run for. The timeout is only enforced if `timeout`
    /// is `true`.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    #[builder(default, setter(strip_option))]
    pub max_duration: Option<Duration>,

    #[cfg(feature = "worker-sidekiq")]
    #[serde(flatten, default)]
    #[builder(default, setter(strip_option))]
    pub sidekiq: Option<SidekiqWorkerConfig>,

    #[cfg(feature = "worker-pg")]
    #[serde(flatten, default)]
    #[builder(default, setter(strip_option))]
    pub pg: Option<PgWorkerConfig>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SidekiqWorkerConfig {
    /// See <https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html#method.disable_argument_coercion>
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub disable_argument_coercion: Option<bool>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct PgWorkerConfig {
    /// The action to take when a job in the queue completes successfully.
    #[serde(default)]
    pub success_action: Option<CompletedAction>,

    /// The action to take when a job in the queue fails and has no more retry attempts.
    #[serde(default)]
    pub failure_action: Option<CompletedAction>,
}

/// Action to take when a job completes processing, either by being processed successfully, or by
/// running out of retry attempts.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CompletedAction {
    /// Move the message to the queue's archive table.
    Archive,
    /// Delete the message.
    Delete,
}

// Todo: add on_success/on_failure handlers?
#[async_trait]
pub trait Worker<S, Args>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    type Error: std::error::Error + Send + Sync;
    type Enqueuer: Enqueuer + Send + Sync;

    /// The name of the worker. This will be encoded in the job data when it's enqueued the backing
    /// database (Redis/Postgres), and used to identify which type should handle a job when it's
    /// fetched from the queue. Therefore, it should be unique across the app, and care should be
    /// taken when refactoring.
    ///
    /// By default, [`Self::name`] returns the name of the type that implements the [`Worker`]
    /// trait. See [`types::simple_type_name`].
    ///
    /// This is not included in the [`EnqueueConfig`] because [`EnqueueConfig`] is included in
    /// the [`crate::config::AppConfig`] to allow defining defaults for the config values, but
    /// the name needs to be specified separately for each [`Worker`].
    fn name() -> String
    where
        Self: Sized,
    {
        types::simple_type_name::<Self>()
    }

    /// Get worker-specific configuration options to use when enqueuing a job. Any value not
    /// provided in the returned [`WorkerConfig`] will fall back to the value from the
    /// [`crate::config::AppConfig`].
    ///
    /// The [`Worker::enqueue_config`] method will be called when enqueuing a job for the worker.
    fn enqueue_config(_state: &S) -> EnqueueConfig {
        EnqueueConfig::default()
    }

    /// Get worker-specific configuration options to use when handling a job. Any value not provided
    /// in the returned [`WorkerConfig`] will fall back to the value from the
    /// [`crate::config::AppConfig`].
    ///
    /// The [`Worker::worker_config`] method will be called once for each worker when it is
    /// registered, and the config will be stored by the [`Processor`] to be used when the worker
    /// handles a job.
    fn worker_config(&self, _state: &S) -> WorkerConfig {
        WorkerConfig::default()
    }

    #[instrument(skip_all)]
    async fn enqueue(state: &S, args: &Args) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue::<Self, S, Args, Self::Error>(state, args).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_delayed(
        state: &S,
        args: &Args,
        delay: Duration,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue_delayed::<Self, S, Args, Self::Error>(state, args, delay).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch(
        state: &S,
        args: &[Args],
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue_batch::<Self, S, Args, Self::Error>(state, args).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch_delayed(
        state: &S,
        args: &[Args],
        delay: Duration,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue_batch_delayed::<Self, S, Args, Self::Error>(state, args, delay)
            .await?;
        Ok(())
    }

    async fn handle(&self, state: &S, args: &Args) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::service::worker::Worker;
    use crate::util;
    use crate::util::types;
    use insta::_macro_support::assert_snapshot;
    use insta::assert_debug_snapshot;
    use serde_derive::{Deserialize, Serialize};
    use std::time::Duration;

    #[derive(Serialize, Deserialize)]
    struct FooWorkerArgs {
        foo: String,
    }

    struct FooWorker;

    #[async_trait::async_trait]
    impl Worker<AppContext, FooWorkerArgs> for FooWorker {
        type Error = crate::error::Error;

        #[cfg_attr(coverage_nightly, coverage(off))]
        async fn handle(
            &self,
            state: &AppContext,
            args: &FooWorkerArgs,
        ) -> Result<(), Self::Error> {
            todo!()
        }
    }

    #[fixture]
    #[once]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn context() -> AppContext {
        let mut config = AppConfig::test(None).unwrap();
        AppContext::test(Some(config), None, None).unwrap()
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn enqueue_config(context: AppContext) {
        let enqueue_config = FooWorker::enqueue_config(&context);
        assert_debug_snapshot!(enqueue_config);
    }
}
