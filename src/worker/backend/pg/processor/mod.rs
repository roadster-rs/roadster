use crate::app::context::AppContext;
use crate::config::AppConfig;
use crate::config::service::worker::{BalanceStrategy, StaleCleanUpBehavior};
use crate::error::RoadsterResult;
use crate::worker::backend::pg::processor::builder::PeriodicArgsJson;
use crate::worker::config::{CompletedAction, failure_action, retry_delay, success_action};
use crate::worker::job::{Job, JobMetadata};
use crate::worker::{EnqueueConfig, Worker, WorkerConfig};
use axum_core::extract::FromRef;
use builder::PgProcessorBuilder;
use chrono::{DateTime, TimeDelta, Utc};
use cron::Schedule;
use itertools::Itertools;
use pgmq::{PGMQueue, PgmqError};
use serde::{Deserialize, Serialize};
use sqlx::Error;
use sqlx::error::ErrorKind;
use std::cmp::{Ordering, max};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::task::JoinSet;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};

pub mod builder;

pub(crate) const PERIODIC_QUEUE_NAME: &str = "periodic";

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PgProcessorError {
    /// The provided [`Worker`] was already registered. Contains the [`Worker::name`]
    /// of the provided worker.
    #[error("The provided `Worker` was already registered: `{0}`")]
    AlreadyRegistered(String),

    /// The provided [`Worker`] was already registered. Contains the [`Worker::name`]
    /// of the provided worker.
    #[error(
        "The provided periodic worker job was already registered. Worker: `{0}`, schedule: `{1}`, args: `{2}`"
    )]
    AlreadyRegisteredPeriodic(String, Schedule, serde_json::Value),

    #[error("No queue configured for worker `{0}`.")]
    NoQueue(String),

    #[error("{0}")]
    InvalidBalanceStrategy(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Clone)]
#[non_exhaustive]
pub struct PgProcessor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Arc<PgProcessorInner<S>>,
}

#[non_exhaustive]
pub(crate) struct PgProcessorInner<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    queues: BTreeSet<String>,
    workers: BTreeMap<String, WorkerWrapper<S>>,
    periodic_workers: BTreeSet<PeriodicArgsJson>,
}

impl<S> PgProcessor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(inner: PgProcessorInner<S>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn builder(state: &S) -> PgProcessorBuilder<S> {
        PgProcessorBuilder::new(state)
    }

    pub async fn before_run(&self, state: &S) -> RoadsterResult<()> {
        let context = AppContext::from_ref(state);
        if context
            .config()
            .service
            .worker
            .pg
            .custom
            .common
            .balance_strategy
            == BalanceStrategy::None
            && self.shared_queues(context.config()).len() > 1
        {
            return Err(PgProcessorError::InvalidBalanceStrategy(format!(
                "{:?} is not supported when more than one shared queue is enabled.",
                BalanceStrategy::None
            ))
            .into());
        }

        self.initialize_queues().await?;
        self.initialize_periodic(state).await?;
        Ok(())
    }

    /// Ensures all of the workers' queues' tables exist in the Postgres database.
    async fn initialize_queues(&self) -> RoadsterResult<()> {
        let context = AppContext::from_ref(&self.inner.state);
        for queue in self.inner.queues.iter() {
            context.pgmq().create(queue).await?;
        }
        Ok(())
    }

    /// Initialize the periodic queue tables and enqueue the periodic jobs in the queue.
    async fn initialize_periodic(&self, state: &S) -> RoadsterResult<()> {
        let context = AppContext::from_ref(state);

        // Create the queue's tables
        context.pgmq().create(PERIODIC_QUEUE_NAME).await?;
        // Create a unique index on the periodic job hash. This ensures we don't enqueue duplicate
        // periodic jobs.
        sqlx::query!(
            r#"CREATE UNIQUE INDEX IF NOT EXISTS roadster_periodic_hash_idx ON pgmq.q_periodic USING btree ((message->'metadata'->'periodic'->'hash'))"#
        ).execute(&context.pgmq().connection).await?;

        let periodic_config = &context.config().service.worker.pg.custom.custom.periodic;

        let periodic_jobs = self
            .inner
            .periodic_workers
            .iter()
            .map(Job::from)
            .collect_vec();

        match periodic_config.stale_cleanup {
            StaleCleanUpBehavior::Manual => {}
            StaleCleanUpBehavior::AutoCleanAll => {
                let rows_affected = context.pgmq().purge(PERIODIC_QUEUE_NAME).await?;
                info!(
                    count = rows_affected,
                    "Deleted all previously registered periodic jobs"
                );
            }
            StaleCleanUpBehavior::AutoCleanStale => {
                let current_job_hashes = periodic_jobs
                    .iter()
                    .filter_map(|job| {
                        job.metadata.periodic.as_ref().map(|periodic| {
                            serde_json::Value::Number(serde_json::Number::from(periodic.hash))
                        })
                    })
                    .collect_vec();
                let result = sqlx::query!(
                    r#"DELETE FROM pgmq.q_periodic where message->'metadata'->'periodic'->'hash' != ALL($1)"#,
                    current_job_hashes.as_slice()
                ).execute(&context.pgmq().connection).await?;
                info!(
                    count = result.rows_affected(),
                    "Deleted stale periodic jobs"
                )
            }
        }

        for job in periodic_jobs.iter() {
            if let Some(schedule) = job
                .metadata
                .periodic
                .as_ref()
                .map(|periodic| &periodic.schedule)
            {
                let delay = periodic_next_run_delay(schedule, None);
                let result = context
                    .pgmq()
                    .send_delay(PERIODIC_QUEUE_NAME, job, delay.as_secs())
                    .await;

                match result {
                    Ok(_) => Ok(()),
                    Err(PgmqError::DatabaseError(Error::Database(err))) => match err.kind() {
                        // We use a unique index constraint to ensure we don't enqueue duplicate periodic
                        // jobs, so we ignore `UniqueViolation` errors, but allow all other errors
                        // to be returned.
                        ErrorKind::UniqueViolation => Ok(()),
                        _ => Err(PgmqError::DatabaseError(Error::Database(err))),
                    },
                    Err(err) => Err(err),
                }?;
            } else {
                return Err(
                    crate::error::worker::EnqueueError::PeriodicJobMissingSchedule(
                        job.metadata.worker_name.clone(),
                        job.args.clone(),
                    )
                    .into(),
                );
            }
        }

        Ok(())
    }

    pub(crate) fn queues(&self) -> &BTreeSet<String> {
        &self.inner.queues
    }

    pub async fn run(self, _state: &S, cancellation_token: CancellationToken) {
        let mut join_set = JoinSet::new();

        let context = AppContext::from_ref(&self.inner.state);
        let worker_config = &context.config().service.worker.pg.custom;
        let dedicated_queues = &worker_config.common.queue_config;
        let shared_queues = self.shared_queues(context.config());

        if !shared_queues.is_empty() {
            let total_worker_tasks = worker_config.common.num_workers;
            for worker_num in 0..total_worker_tasks {
                join_set.spawn(self.clone().process_queues(
                    cancellation_token.clone(),
                    worker_num + 1,
                    total_worker_tasks,
                    shared_queues.clone(),
                ));
            }
        }

        for (queue, config) in dedicated_queues {
            let total_worker_tasks = config.num_workers.unwrap_or_default();
            for worker_num in 0..total_worker_tasks {
                join_set.spawn(self.clone().process_queues(
                    cancellation_token.clone(),
                    worker_num + 1,
                    total_worker_tasks,
                    vec![queue.to_owned()],
                ));
            }
        }

        if worker_config.custom.periodic.enable && !self.inner.periodic_workers.is_empty() {
            join_set.spawn(self.clone().process_periodic(cancellation_token.clone()));
        }

        while let Some(result) = join_set.join_next().await {
            // Once any of the tasks finish, cancel the cancellation token to ensure
            // the processor and the app shut down gracefully.
            cancellation_token.cancel();
            if let Err(join_err) = result {
                error!(
                    "An error occurred when trying to join on one of the processor's workers. Error: {join_err}"
                );
            }
        }
    }

    async fn process_queues(
        self,
        cancellation_token: CancellationToken,
        worker_task_num: u32,
        total_worker_tasks: u32,
        queues: Vec<String>,
    ) {
        let num_queues = queues.len();
        let queue_name = if num_queues == 1 {
            queues.first().cloned()
        } else {
            None
        };

        let mut queues: BinaryHeap<QueueItem> = queues
            .into_iter()
            .map(|name| QueueItem {
                name,
                next_fetch: Utc::now(),
            })
            .collect();

        let context = AppContext::from_ref(&self.inner.state);
        let default_worker_config = &context.config().service.worker.worker_config;
        let default_max_duration = default_worker_config.max_duration;
        let default_view_timeout = default_max_duration
            .as_ref()
            .and_then(|duration| duration.as_secs().try_into().ok());

        let empty_delay = context
            .config()
            .service
            .worker
            .pg
            .custom
            .custom
            .queue_fetch_config
            .as_ref()
            .and_then(|config| config.empty_delay)
            .unwrap_or_default();

        let error_delay = context
            .config()
            .service
            .worker
            .pg
            .custom
            .custom
            .queue_fetch_config
            .as_ref()
            .and_then(|config| config.error_delay)
            .unwrap_or_default();

        let pgmq = context.pgmq();
        loop {
            while let Some(mut queue) = queues.peek_mut() {
                {
                    let diff = max(TimeDelta::zero(), queue.next_fetch - Utc::now());
                    let duration = diff.to_std().unwrap_or_else(|_| Duration::from_secs(0));
                    tokio::select! {
                        // `biased` ensures that the cancellation token is polled first
                        biased;

                        _ = cancellation_token.cancelled() => {
                            info!(
                                worker_task_num,
                                total_worker_tasks,
                                num_queues,
                                queue = queue_name,
                                "Exiting processor worker loop"
                            );
                            return;

                        },
                        _ = sleep(duration) => (),
                    }
                }

                /*
                Deserialize to `serde_json::Value` first. We do this because pgmq does not return
                the message id if an error occurs when deserializing a custom type. So, if there
                is a deserialization error, we wouldn't be able to update the view timeout of
                the message and it will stay at the front of the queue indefinitely, blocking
                all other work. Deserializing to `serde_json::Value` first will avoid all
                deserialization errors (aside from those due to corrupted date, which should be
                rare). Then, we can separately handle any deserialization errors ourselves.
                 */
                let msg = match pgmq
                    .read::<serde_json::Value>(&queue.name, default_view_timeout)
                    .await
                {
                    Ok(Some(msg)) => msg,
                    Ok(None) => {
                        queue.next_fetch = Utc::now() + empty_delay;
                        continue;
                    }
                    Err(err) => {
                        error!(
                            queue = queue.name,
                            "An error occurred while reading from pgmq: {err}"
                        );
                        queue.next_fetch = Utc::now() + error_delay;
                        continue;
                    }
                };

                let job: Job = match serde_json::from_value(msg.message) {
                    Ok(job) => job,
                    Err(err) => {
                        error!(
                            msg_id = msg.msg_id,
                            read_count = msg.read_ct,
                            queue = queue.name,
                            "An error occurred while deserializing message from pgmq: {err}"
                        );
                        self.retry(
                            pgmq,
                            &queue,
                            None,
                            msg.msg_id,
                            msg.read_ct,
                            context.config(),
                            None,
                        )
                        .await;

                        queue.next_fetch = Utc::now();
                        continue;
                    }
                };

                let worker = if let Some(worker) = self.inner.workers.get(&job.metadata.worker_name)
                {
                    worker
                } else {
                    error!(
                        msg_id = msg.msg_id,
                        read_count = msg.read_ct,
                        queue = queue.name,
                        worker_name = job.metadata.worker_name,
                        "Unable to handle job, worker not registered"
                    );
                    self.retry(
                        pgmq,
                        &queue,
                        Some(&job.metadata),
                        msg.msg_id,
                        msg.read_ct,
                        context.config(),
                        None,
                    )
                    .await;

                    queue.next_fetch = Utc::now();
                    continue;
                };

                // Update the view timeout to match the max duration of the worker, if it's
                // different from the default.
                let max_duration = if let Some((worker_max, default_max)) =
                    worker.worker_config.max_duration.zip(default_max_duration)
                {
                    if worker_max != default_max {
                        Some(worker_max)
                    } else {
                        None
                    }
                } else {
                    worker.worker_config.max_duration
                };
                if let Some(delay) = max_duration {
                    self.update_job_view_timeout(
                        pgmq,
                        &queue,
                        Some(&job.metadata),
                        msg.msg_id,
                        msg.read_ct,
                        delay,
                    )
                    .await;
                }

                let result = worker.handle(&self.inner.state, job.args).await;

                if let Err(err) = result {
                    error!(
                        msg_id = msg.msg_id,
                        read_count = msg.read_ct,
                        queue = queue.name,
                        worker_name = job.metadata.worker_name,
                        "An error occurred while handling a job: {err}"
                    );
                    self.retry(
                        pgmq,
                        &queue,
                        Some(&job.metadata),
                        msg.msg_id,
                        msg.read_ct,
                        context.config(),
                        Some(worker),
                    )
                    .await;
                } else {
                    let action = success_action(context.config(), worker.worker_config.pg.as_ref());
                    self.job_completed(
                        pgmq,
                        &queue,
                        Some(&job.metadata),
                        msg.msg_id,
                        msg.read_ct,
                        action,
                    )
                    .await;
                }

                queue.next_fetch = Utc::now();
            }
        }
    }

    async fn process_periodic(self, cancellation_token: CancellationToken) {
        let context = AppContext::from_ref(&self.inner.state);
        let default_enqueue_config = &context.config().service.worker.enqueue_config;
        let default_worker_config = &context.config().service.worker.worker_config;
        let default_max_duration = default_worker_config.max_duration;
        let default_view_timeout = default_max_duration
            .as_ref()
            .and_then(|duration| duration.as_secs().try_into().ok());

        let empty_delay = context
            .config()
            .service
            .worker
            .pg
            .custom
            .custom
            .queue_fetch_config
            .as_ref()
            .and_then(|config| config.empty_delay)
            .unwrap_or_default();

        let error_delay = context
            .config()
            .service
            .worker
            .pg
            .custom
            .custom
            .queue_fetch_config
            .as_ref()
            .and_then(|config| config.error_delay)
            .unwrap_or_default();

        let mut next_fetch = Utc::now();

        let pgmq = context.pgmq();
        loop {
            {
                let diff = max(TimeDelta::zero(), next_fetch - Utc::now());
                let duration = diff.to_std().unwrap_or_else(|_| Duration::from_secs(0));
                tokio::select! {
                    // `biased` ensures that the cancellation token is polled first
                    biased;

                    _ = cancellation_token.cancelled() => {
                        info!("Exiting processor periodic worker loop");
                        return;
                    },
                    _ = sleep(duration) => (),
                }
            }

            /*
            Deserialize to `serde_json::Value` first. We do this because pgmq does not return
            the message id if an error occurs when deserializing a custom type. So, if there
            is a deserialization error, we wouldn't be able to update the view timeout of
            the message and it will stay at the front of the queue indefinitely, blocking
            all other work. Deserializing to `serde_json::Value` first will avoid all
            deserialization errors (aside from those due to corrupted date, which should be
            rare). Then, we can separately handle any deserialization errors ourselves.
             */
            let msg = match pgmq
                .read::<serde_json::Value>(PERIODIC_QUEUE_NAME, default_view_timeout)
                .await
            {
                Ok(Some(msg)) => msg,
                Ok(None) => {
                    next_fetch = Utc::now() + empty_delay;
                    continue;
                }
                Err(err) => {
                    error!(
                        queue = PERIODIC_QUEUE_NAME,
                        "An error occurred while reading from pgmq: {err}"
                    );
                    next_fetch = Utc::now() + error_delay;
                    continue;
                }
            };

            let job: Job = match serde_json::from_value(msg.message) {
                Ok(job) => job,
                Err(err) => {
                    error!(
                        msg_id = msg.msg_id,
                        read_count = msg.read_ct,
                        queue = PERIODIC_QUEUE_NAME,
                        "An error occurred while deserializing message from pgmq: {err}"
                    );
                    // For periodic jobs, we simply delete the failing msg. It will
                    // be re-enqueued the next time the app starts
                    if let Err(err) = context.pgmq().delete(PERIODIC_QUEUE_NAME, msg.msg_id).await {
                        error!(
                            msg_id = msg.msg_id,
                            read_count = msg.read_ct,
                            queue = PERIODIC_QUEUE_NAME,
                            "An error occurred while deleting periodic job: {err}"
                        );
                        next_fetch = Utc::now() + error_delay;
                    } else {
                        next_fetch = Utc::now();
                    }
                    continue;
                }
            };

            let worker = self.inner.workers.get(&job.metadata.worker_name);
            let queue = worker
                .and_then(|worker| worker.enqueue_config.queue.as_ref())
                .or(default_enqueue_config.queue.as_ref());
            let periodic = job.metadata.periodic.as_ref();

            let (worker, queue, periodic) = if let Some(((worker, queue), periodic)) =
                worker.zip(queue).zip(periodic)
            {
                (worker, queue, periodic)
            } else {
                error!(
                    msg_id = msg.msg_id,
                    read_count = msg.read_ct,
                    worker_name = job.metadata.worker_name,
                    queue,
                    ?periodic,
                    "Unable to enqueue job; worker not registered, no queue configured, or no periodic metadata configured"
                );
                // For periodic jobs, we simply delete the failing msg. It will
                // be re-enqueued the next time the app starts
                if let Err(err) = context.pgmq().delete(PERIODIC_QUEUE_NAME, msg.msg_id).await {
                    error!(
                        msg_id = msg.msg_id,
                        read_count = msg.read_ct,
                        queue = PERIODIC_QUEUE_NAME,
                        "An error occurred while deleting periodic job: {err}"
                    );
                    next_fetch = Utc::now() + error_delay;
                } else {
                    next_fetch = Utc::now();
                }
                continue;
            };

            let job_to_enqueue = Job::builder()
                .args(job.args.clone())
                .metadata(
                    JobMetadata::builder()
                        .worker_name(job.metadata.worker_name)
                        .build(),
                )
                .build();
            if let Err(err) = context.pgmq().send(queue, &job_to_enqueue).await {
                error!(
                    msg_id = msg.msg_id,
                    read_count = msg.read_ct,
                    worker_name = worker.name,
                    queue,
                    "An error occurred while enqueuing periodic job: {err}"
                );

                next_fetch = Utc::now() + error_delay;
                continue;
            }

            let delay = periodic_next_run_delay(&periodic.schedule, None);
            if let Err(err) = pgmq
                .set_vt::<serde_json::Value>(PERIODIC_QUEUE_NAME, msg.msg_id, Utc::now() + delay)
                .await
            {
                error!(
                    msg_id = msg.msg_id,
                    read_count = msg.read_ct,
                    queue = PERIODIC_QUEUE_NAME,
                    worker_name = worker.name,
                    ?delay,
                    "An error occurred while updating periodic job's view timeout: {err}"
                );
                next_fetch = Utc::now() + error_delay;
                continue;
            }

            next_fetch = Utc::now();
        }
    }

    fn shared_queues(&self, config: &AppConfig) -> Vec<String> {
        let worker_config = &config.service.worker.pg.custom;
        worker_config
            .common
            .queues
            .as_ref()
            .unwrap_or(&self.inner.queues)
            .iter()
            .filter(|queue| !worker_config.common.queue_config.contains_key(*queue))
            .map(|queue| queue.to_owned())
            .collect_vec()
    }

    #[instrument(skip_all)]
    async fn retry(
        &self,
        pgmq: &PGMQueue,
        queue: &QueueItem,
        job_metadata: Option<&JobMetadata>,
        msg_id: i64,
        read_count: i32,
        app_config: &AppConfig,
        worker: Option<&WorkerWrapper<S>>,
    ) {
        if let Some(delay) = retry_delay(
            app_config,
            worker.and_then(|worker| worker.worker_config.retry_config.as_ref()),
            read_count,
        ) {
            // If the job can retry, update its view timeout by the calculated delay.
            self.update_job_view_timeout(pgmq, queue, job_metadata, msg_id, read_count, delay)
                .await;
        } else {
            // Otherwise, perform the failure action for the worker.
            let action = failure_action(
                app_config,
                worker.and_then(|worker| worker.worker_config.pg.as_ref()),
            );
            self.job_completed(pgmq, queue, job_metadata, msg_id, read_count, action)
                .await;
        }
    }

    #[instrument(skip_all)]
    async fn update_job_view_timeout(
        &self,
        pgmq: &PGMQueue,
        queue: &QueueItem,
        job_metadata: Option<&JobMetadata>,
        msg_id: i64,
        read_count: i32,
        delay: Duration,
    ) {
        if let Err(err) = pgmq
            .set_vt::<serde_json::Value>(&queue.name, msg_id, Utc::now() + delay)
            .await
        {
            error!(
                msg_id,
                read_count,
                queue = queue.name,
                worker_name = job_metadata.map(|metadata| &metadata.worker_name),
                "An error occurred while updating job's view timeout: {err}"
            );
        }
    }

    #[instrument(skip_all)]
    async fn job_completed(
        &self,
        pgmq: &PGMQueue,
        queue: &QueueItem,
        job_metadata: Option<&JobMetadata>,
        msg_id: i64,
        read_count: i32,
        action: &CompletedAction,
    ) {
        debug!(
            msg_id,
            read_count,
            queue = queue.name,
            worker_name = job_metadata.map(|metadata| &metadata.worker_name),
            ?action,
            "Performing completed action for a job"
        );

        let result = match action {
            CompletedAction::Archive => pgmq.archive(&queue.name, msg_id).await,
            CompletedAction::Delete => pgmq.delete(&queue.name, msg_id).await,
        };

        if let Err(err) = result {
            error!(
                msg_id,
                read_count,
                queue = queue.name,
                worker_name = job_metadata.map(|metadata| &metadata.worker_name),
                ?action,
                "An error occurred while performing completed action for a job: {err}"
            );
        }
    }
}

struct QueueItem {
    name: String,
    next_fetch: DateTime<Utc>,
}

impl Eq for QueueItem {}

impl PartialEq<Self> for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.next_fetch == other.next_fetch
    }
}

impl PartialOrd<Self> for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // This is intentionally reversed so that `QueueItem` forms a min heap when used in
        // a binary heap.
        other.next_fetch.cmp(&self.next_fetch)
    }
}

fn periodic_next_run_delay(schedule: &Schedule, now: Option<DateTime<Utc>>) -> Duration {
    let now = now.unwrap_or_else(Utc::now);
    let next_run = schedule.after(&now).next().unwrap_or(now);
    let diff = max(TimeDelta::zero(), next_run - now);
    diff.to_std().unwrap_or_else(|_| Duration::from_secs(0))
}

type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
            serde_json::Value,
        ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

struct WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    name: String,
    #[allow(dead_code)]
    enqueue_config: EnqueueConfig,
    worker_config: WorkerConfig,
    worker_fn: WorkerFn<S>,
}

impl<S> WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn new<W, Args, E>(state: &S, worker: W, enqueue_config: EnqueueConfig) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let worker = Arc::new(worker);

        Ok(Self {
            name: W::name(),
            enqueue_config,
            worker_config: worker.worker_config(state),
            worker_fn: Box::new(move |state: &S, args: serde_json::Value| {
                let worker = worker.clone();
                Box::pin(async move {
                    let args: Args = serde_json::from_value(args)
                        .map_err(crate::error::worker::DequeueError::Serde)?;

                    match worker.clone().handle(state, args).await {
                        Ok(_) => Ok(()),
                        Err(err) => Err(crate::error::Error::from(
                            crate::error::worker::WorkerError::Handle(W::name(), Box::new(err)),
                        )),
                    }
                })
            }),
        })
    }

    #[instrument(skip_all)]
    async fn handle(&self, state: &S, args: serde_json::Value) -> RoadsterResult<()> {
        let inner = (self.worker_fn)(state, args);

        let context = AppContext::from_ref(state);
        let timeout = self
            .worker_config
            .timeout
            .or(context.config().service.worker.worker_config.timeout)
            .unwrap_or_default();

        let max_duration = if timeout {
            self.worker_config.max_duration.or(context
                .config()
                .service
                .worker
                .worker_config
                .max_duration)
        } else {
            None
        };

        if let Some(max_duration) = max_duration {
            tokio::time::timeout(max_duration, inner)
                .await
                .map_err(|err| {
                    error!(
                        worker = self.name,
                        max_duration = max_duration.as_secs(),
                        %err,
                        "Worker timed out"
                    );
                    crate::error::worker::WorkerError::Timeout(
                        self.name.clone(),
                        max_duration,
                        Box::new(err),
                    )
                })?
        } else {
            inner.await
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use chrono::Utc;
    use cron::Schedule;
    use insta::assert_debug_snapshot;
    use std::str::FromStr;

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn periodic_queue_name() {
        assert_eq!(super::PERIODIC_QUEUE_NAME, "periodic");
    }

    mod queue_item {
        use crate::worker::backend::pg::processor::QueueItem;
        use chrono::Utc;
        use std::collections::BinaryHeap;
        use std::time::Duration;

        #[test]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn min_heap() {
            let now = Utc::now();
            let mut items = BinaryHeap::new();
            items.push(QueueItem {
                name: "a".to_owned(),
                next_fetch: now + Duration::from_secs(1),
            });
            items.push(QueueItem {
                name: "b".to_owned(),
                next_fetch: now,
            });
            items.push(QueueItem {
                name: "c".to_owned(),
                next_fetch: now + Duration::from_secs(10),
            });

            assert_eq!(items.pop().unwrap().name, "b");
            assert_eq!(items.pop().unwrap().name, "a");
            assert_eq!(items.pop().unwrap().name, "c");
        }

        #[test]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn peek_mut_change_order() {
            let now = Utc::now();
            let mut items = BinaryHeap::new();
            items.push(QueueItem {
                name: "a".to_owned(),
                next_fetch: now,
            });
            items.push(QueueItem {
                name: "b".to_owned(),
                next_fetch: now + Duration::from_secs(1),
            });

            if let Some(mut item) = items.peek_mut() {
                item.next_fetch = now + Duration::from_secs(10);
            }

            assert_eq!(items.pop().unwrap().name, "b");
            assert_eq!(items.pop().unwrap().name, "a");
        }
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn periodic_next_run_delay() {
        let now = DateTime::<Utc>::from_timestamp(1751701139, 0).unwrap();
        let schedule = Schedule::from_str("* 12 * * * *").unwrap();
        let delay = super::periodic_next_run_delay(&schedule, Some(now));
        assert_debug_snapshot!(delay);
    }
}
