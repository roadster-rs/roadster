use crate::config::CustomConfig;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::time::Duration;
use validator::Validate;

/// Worker configuration options to use when enqueuing a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`crate::worker::Worker::enqueue_config`] method.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, bon::Builder)]
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
    #[builder(into)]
    pub queue: Option<String>,

    #[serde(flatten, default)]
    #[builder(default)]
    #[validate(nested)]
    pub custom: CustomConfig,
}

/// Worker configuration options to use when handling a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`crate::worker::Worker::worker_config`] method.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, bon::Builder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig {
    /// True if Roadster should enforce a timeout on the app's workers. The default duration of
    /// the timeout can be configured with the `max-duration` option.
    #[serde(default)]
    pub timeout: Option<bool>,

    /// The maximum duration workers should run for. The timeout is only enforced if `timeout`
    /// is `true`.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub max_duration: Option<Duration>,

    /// The worker retry configuration. If no configuration is provided, either in the app's config
    /// or for the [`crate::worker::Worker`], the worker will not retry.
    #[serde(flatten, default)]
    pub retry_config: Option<RetryConfig>,

    #[cfg(feature = "worker-sidekiq")]
    #[serde(default)]
    pub sidekiq: Option<SidekiqWorkerConfig>,

    #[cfg(feature = "worker-pg")]
    #[serde(default)]
    pub pg: Option<PgWorkerConfig>,

    #[serde(flatten, default)]
    #[builder(default)]
    #[validate(nested)]
    pub custom: CustomConfig,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, bon::Builder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SidekiqWorkerConfig {
    /// See <https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html#method.disable_argument_coercion>
    #[serde(default)]
    pub disable_argument_coercion: Option<bool>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, bon::Builder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct RetryConfig {
    /// The maximum number of times a job should be retried on failure.
    #[serde(default)]
    pub max_retries: Option<usize>,

    /// The delay between retries. If a [`BackoffStrategy`] is provided, this will be used as the
    /// base delay of the backoff calculation.
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded delay.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub delay: Option<Duration>,

    /// An offset to add to the base `delay` to add jitter to the delay to avoid a "thundering herd"
    /// problem. A random value between 0 and the provided [`Duration`] will be added to the
    /// `base` delay before performing any provided [`BackoffStrategy`].
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded delay.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub delay_offset: Option<Duration>,

    /// The maximum duration to delay the retry.
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded delay.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub max_delay: Option<Duration>,

    /// The retry delay backoff algorithm to use.
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded exponential backoff strategy.
    #[serde(default)]
    pub backoff_strategy: Option<BackoffStrategy>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum BackoffStrategy {
    #[default]
    Exponential,
    Linear,
    None,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, bon::Builder)]
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
