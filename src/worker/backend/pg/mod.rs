//! Background task queue service backed by Postgres using [pgmq](https://docs.rs/pgmq).

use crate::config::AppConfig;
use crate::worker::config::{BackoffStrategy, CompletedAction, PgWorkerConfig, RetryConfig};
use rand::Rng;
use std::cmp::min;
use std::sync::OnceLock;
use std::time::Duration;

pub mod enqueue;
pub(crate) mod periodic_job;
pub mod processor;

static DEFAULT_COMPLETED_ACTION: OnceLock<CompletedAction> = OnceLock::new();

/// Action to take if a job succeeds.
pub(crate) fn success_action<'a>(
    app_config: &'a AppConfig,
    worker_config: Option<&'a PgWorkerConfig>,
) -> &'a CompletedAction {
    worker_config
        .and_then(|config| config.success_action.as_ref())
        .or(app_config
            .service
            .worker
            .worker_config
            .pg
            .as_ref()
            .and_then(|config| config.success_action.as_ref()))
        .unwrap_or(DEFAULT_COMPLETED_ACTION.get_or_init(|| CompletedAction::Delete))
}

/// Action to take if a job fails.
pub(crate) fn failure_action<'a>(
    app_config: &'a AppConfig,
    worker_config: Option<&'a PgWorkerConfig>,
) -> &'a CompletedAction {
    worker_config
        .and_then(|config| config.failure_action.as_ref())
        .or(app_config
            .service
            .worker
            .worker_config
            .pg
            .as_ref()
            .and_then(|config| config.failure_action.as_ref()))
        .unwrap_or(DEFAULT_COMPLETED_ACTION.get_or_init(|| CompletedAction::Archive))
}

/// Calculate the retry delay based on the provided [`RetryConfig`]s and the number of attempts.
///
/// Return None if it should not retry.
pub(crate) fn retry_delay(
    app_config: &AppConfig,
    worker_retry_config: Option<&RetryConfig>,
    attempt_num: i32,
) -> Option<Duration> {
    let attempt_u32 = u32::try_from(attempt_num).ok()?;
    let attempt_num = usize::try_from(attempt_u32).unwrap_or(usize::MAX);

    let default_retry_config = app_config
        .service
        .worker
        .worker_config
        .retry_config
        .as_ref();

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
        Some(delay_offset) => delay.saturating_add(Duration::from_secs(
            rand::rng().random_range(0..delay_offset.as_secs()),
        )),
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
        BackoffStrategy::Exponential => Duration::from_secs(
            delay
                .as_secs()
                .saturating_mul(2u64.saturating_pow(attempt_u32)),
        ),
        BackoffStrategy::Linear => delay.saturating_mul(attempt_u32),
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

// todo
#[cfg(test)]
mod tests {}
