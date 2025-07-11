//! Background task queue service backed by Postgres using [pgmq](https://docs.rs/pgmq).

use crate::config::AppConfig;
use crate::worker::config::{CompletedAction, PgWorkerConfig};
use std::sync::OnceLock;

pub mod enqueue;
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
