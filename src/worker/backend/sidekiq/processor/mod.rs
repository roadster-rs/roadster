use crate::app::context::AppContext;
use crate::config::service::worker::StaleCleanUpBehavior;
use crate::error::RoadsterResult;
use crate::util::redis::RedisCommands;
use crate::worker::backend::sidekiq::processor::builder::SidekiqProcessorBuilder;
use crate::worker::{
    PeriodicArgsJson, RegisterSidekiqFn, RegisterSidekiqPeriodicFn, WorkerWrapper,
};
use axum_core::extract::FromRef;
use cron::Schedule;
use itertools::Itertools;
use sidekiq::periodic;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, Mutex};
use thiserror::Error;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

pub mod builder;

const PERIODIC_KEY: &str = "periodic";

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SidekiqProcessorError {
    /// The provided [`Worker`] was already registered. Contains the [`Worker::name`]
    /// of the provided worker.
    #[error("The provided `Worker` name was already registered: `{0}`")]
    AlreadyRegistered(String),

    /// A [`Worker`] was previously registered that has the same name but is a different type.
    #[error("The provided `Worker` name was already registered for a different type: `{0}`")]
    AlreadyRegisteredWithDifferentType(String),

    /// The provided [`Worker`] was already registered. Contains the [`Worker::name`]
    /// of the provided worker.
    #[error(
        "The provided periodic worker job was already registered. Worker: `{0}`, schedule: `{1}`, args: `{2}`"
    )]
    AlreadyRegisteredPeriodic(String, Schedule, serde_json::Value),

    #[error("No queue configured for worker `{0}`.")]
    NoQueue(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Clone)]
#[non_exhaustive]
pub struct SidekiqProcessor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Arc<SidekiqProcessorInner<S>>,
}

pub(crate) struct WorkerData<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) worker_wrapper: WorkerWrapper<S>,
    pub(crate) register_sidekiq_fn: RegisterSidekiqFn<S>,
    pub(crate) register_sidekiq_periodic_fn: RegisterSidekiqPeriodicFn<S>,
}

#[non_exhaustive]
pub(crate) struct SidekiqProcessorInner<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    // Todo: we may need to register directly on the processor instead of waiting to register
    //  until later, depending on if `RoadsterWorker` needs the `W` type param.
    // todo: store a closure to register the worker in order to keep the type?
    processor: Mutex<Option<::sidekiq::Processor>>,
    // queues: BTreeSet<String>,
    workers: BTreeMap<String, WorkerData<S>>,
    periodic_workers: HashSet<PeriodicArgsJson>,
}

impl<S> SidekiqProcessor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(inner: SidekiqProcessorInner<S>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn builder(state: &S) -> SidekiqProcessorBuilder<S> {
        SidekiqProcessorBuilder::new(state)
    }

    pub async fn before_run(&self, state: &S) -> RoadsterResult<()> {
        self.initialize_periodic(state).await?;

        Ok(())
    }

    /// Initialize the periodic queue tables and enqueue the periodic jobs in the queue.
    async fn initialize_periodic(&self, state: &S) -> RoadsterResult<()> {
        let context = AppContext::from_ref(state);

        let periodic_config = &context
            .config()
            .service
            .worker
            .sidekiq
            .custom
            .custom
            .periodic;

        match periodic_config.stale_cleanup {
            StaleCleanUpBehavior::Manual => {}
            StaleCleanUpBehavior::AutoCleanAll => {
                periodic::destroy_all(context.redis_enqueue().inner.clone()).await?;
                info!("Deleted all previously registered periodic jobs");
            }
            StaleCleanUpBehavior::AutoCleanStale => {
                let mut conn = context.redis_enqueue().get().await?;
                // todo: replace `default` with actual registered periodic jobs
                remove_stale_periodic_jobs(&mut conn, &context, &Default::default()).await?;
            }
        };

        let mut processor = self.inner.processor.lock()?;

        let processor = if let Some(processor) = processor.as_mut() {
            processor
        } else {
            return Err(todo!());
        };

        for periodic_args in self.inner.periodic_workers.iter() {
            if let Some(worker_data) = self.inner.workers.get(&periodic_args.worker_name) {
                (worker_data.register_sidekiq_periodic_fn)(
                    &self.inner.state,
                    processor,
                    worker_data.worker_wrapper.clone(),
                    periodic_args.clone(),
                )
                .await?;
            } else {
                return Err(todo!());
            }
        }

        Ok(())
    }

    pub async fn run(self, _state: &S, cancellation_token: CancellationToken) {
        let processor = {
            match self.inner.processor.lock() {
                Ok(processor) => processor.clone(),
                Err(err) => {
                    error!("Unable to lock ::sidekiq::Processor: {err}");
                    cancellation_token.cancel();
                    return;
                }
            }
        };

        let processor = match processor.clone() {
            Some(processor) => processor,
            None => {
                warn!("No ::sidekiq::Processor available.");
                return;
            }
        };
        let sidekiq_cancel_token = processor.get_cancellation_token();

        let mut join_set = JoinSet::new();
        let token = cancellation_token.clone();
        join_set.spawn(Box::pin(async move {
            token.cancelled().await;
        }));
        let token = sidekiq_cancel_token.clone();
        join_set.spawn(Box::pin(async move {
            token.cancelled().await;
        }));
        join_set.spawn(processor.run());

        while let Some(result) = join_set.join_next().await {
            // Once any of the tasks finish, cancel all the cancellation tokens to ensure
            // the processor and the app shut down gracefully.
            cancellation_token.cancel();
            sidekiq_cancel_token.cancel();
            if let Err(join_err) = result {
                error!(
                    "An error occurred when trying to join on one of the app's tasks. Error: {join_err}"
                );
            }
        }
    }
}

/// Compares the list of periodic jobs that were registered by the app during app startup with
/// the list of periodic jobs in Redis, and removes any that exist in Redis but weren't
/// registered during start up.
///
/// The jobs are only removed if the [worker.sidekiq.periodic.stale-cleanup][crate::config::worker::Periodic]
/// config is set to [auto-clean-stale][StaleCleanUpBehavior::AutoCleanStale].
///
/// This is run after all the app's periodic jobs have been registered.
async fn remove_stale_periodic_jobs<C: RedisCommands>(
    conn: &mut C,
    context: &AppContext,
    registered_periodic_workers: &HashSet<String>,
) -> RoadsterResult<()> {
    let stale_jobs = conn
        .zrange(PERIODIC_KEY.to_string(), 0, -1)
        .await?
        .into_iter()
        .filter(|job| !registered_periodic_workers.contains(job))
        .collect_vec();

    if stale_jobs.is_empty() {
        info!("No stale periodic jobs found");
        return Ok(());
    }

    if context
        .config()
        .service
        .worker
        .sidekiq
        .custom
        .custom
        .periodic
        .stale_cleanup
        == StaleCleanUpBehavior::AutoCleanStale
    {
        info!(count = stale_jobs.len(), "Removing stale periodic jobs",);
        conn.zrem(PERIODIC_KEY.to_string(), stale_jobs.clone())
            .await?;
    } else {
        warn!(count = stale_jobs.len(), "Found stale periodic jobs");
    }

    Ok(())
}
