use crate::config::CustomConfig;
use num_traits::pow;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::cmp::min;
use std::sync::OnceLock;
use std::time::Duration;
use typed_builder::TypedBuilder;
use validator::Validate;

/// Worker configuration options to use when enqueuing a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`crate::worker::Worker::enqueue_config`] method.
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
    #[builder(default, setter(into, strip_option(fallback = queue_opt)))]
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
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig {
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

    /// The worker retry configuration. If no configuration is provided, either in the app's config
    /// or for the [`Worker`], the worker will not retry.
    #[serde(flatten, default)]
    #[builder(default, setter(strip_option))]
    pub retry_config: Option<RetryConfig>,

    #[cfg(feature = "worker-sidekiq")]
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub sidekiq: Option<SidekiqWorkerConfig>,

    #[cfg(feature = "worker-pg")]
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub pg: Option<PgWorkerConfig>,

    #[serde(flatten, default)]
    #[builder(default)]
    #[validate(nested)]
    pub custom: CustomConfig,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SidekiqWorkerConfig {
    /// See <https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html#method.disable_argument_coercion>
    #[serde(default)]
    #[builder(default, setter(strip_option(fallback = disable_argument_coercion_opt)))]
    pub disable_argument_coercion: Option<bool>,
}

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct RetryConfig {
    /// The maximum number of times a job should be retried on failure.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub max_retries: Option<usize>,

    /// The delay between retries. If a [`BackoffStrategy`] is provided, this will be used as the
    /// base delay of the backoff calculation.
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded delay.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    #[builder(default, setter(strip_option))]
    pub delay: Option<Duration>,

    /// An offset to add to the base `delay` to add jitter to the delay to avoid a "thundering herd"
    /// problem. A random value between 0 and the provided [`Duration`] will be added to the
    /// `base` delay before performing any provided [`BackoffStrategy`].
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded delay.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    #[builder(default, setter(strip_option))]
    pub delay_offset: Option<Duration>,

    /// The maximum duration to delay the retry.
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded delay.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    #[builder(default, setter(strip_option))]
    pub max_delay: Option<Duration>,

    /// The retry delay backoff algorithm to use.
    ///
    /// Note: Not all worker backends will use this. For example, the Sidekiq backend does not use
    /// this and instead uses a hard-coded exponential backoff strategy.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
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

/// Calculate the retry delay based on the provided [`RetryConfig`]s and the number of attempts.
///
/// Return None if it should not retry.
pub(crate) fn retry_delay(
    default_retry_config: Option<&RetryConfig>,
    worker_retry_config: Option<&RetryConfig>,
    attempt_num: i32,
) -> Option<Duration> {
    let attempt_num = usize::try_from(attempt_num).unwrap_or_else(|_| usize::MAX);

    let max_retries = worker_retry_config
        .and_then(|config| config.max_retries)
        .or(default_retry_config.and_then(|config| config.max_retries))
        .unwrap_or_default();

    if attempt_num > max_retries {
        return None;
    }

    let delay = worker_retry_config
        .and_then(|config| config.delay)
        .or(default_retry_config.and_then(|config| config.delay));
    let delay = match delay {
        Some(delay) => delay,
        None => {
            return None;
        }
    };

    let delay_offset = worker_retry_config
        .and_then(|config| config.delay_offset)
        .or(default_retry_config.and_then(|config| config.delay_offset));
    let delay = match delay_offset {
        Some(delay_offset) => {
            delay + Duration::from_secs(rand::rng().random_range(0..delay_offset.as_secs()))
        }
        None => delay,
    };

    let backoff_strategy = worker_retry_config
        .and_then(|config| config.backoff_strategy.as_ref())
        .or(default_retry_config.and_then(|config| config.backoff_strategy.as_ref()));
    let backoff_strategy = match backoff_strategy {
        Some(backoff_strategy) => backoff_strategy,
        None => {
            return Some(delay);
        }
    };

    let delay = match backoff_strategy {
        BackoffStrategy::Exponential => Duration::from_secs(pow(delay.as_secs(), attempt_num)),
        BackoffStrategy::Linear => match u32::try_from(attempt_num) {
            Ok(attempt_num) => delay * attempt_num,
            Err(_) => return None,
        },
        BackoffStrategy::None => delay,
    };

    let max_delay = worker_retry_config
        .and_then(|config| config.max_delay.as_ref())
        .or(default_retry_config.and_then(|config| config.max_delay.as_ref()));
    let delay = match max_delay {
        Some(max_delay) => min(*max_delay, delay),
        None => delay,
    };

    Some(delay)
}

static DEFAULT_COMPLETED_ACTION: OnceLock<CompletedAction> = OnceLock::new();

/// Action to take if a job succeeds.
pub(crate) fn success_action<'a>(
    default_config: Option<&'a PgWorkerConfig>,
    worker_config: Option<&'a PgWorkerConfig>,
) -> &'a CompletedAction {
    worker_config
        .and_then(|config| config.success_action.as_ref())
        .or(default_config.and_then(|config| config.success_action.as_ref()))
        .unwrap_or(DEFAULT_COMPLETED_ACTION.get_or_init(|| CompletedAction::Delete))
}

/// Action to take if a job fails.
pub(crate) fn failure_action<'a>(
    default_config: Option<&'a PgWorkerConfig>,
    worker_config: Option<&'a PgWorkerConfig>,
) -> &'a CompletedAction {
    worker_config
        .and_then(|config| config.failure_action.as_ref())
        .or(default_config.and_then(|config| config.failure_action.as_ref()))
        .unwrap_or(DEFAULT_COMPLETED_ACTION.get_or_init(|| CompletedAction::Archive))
}
