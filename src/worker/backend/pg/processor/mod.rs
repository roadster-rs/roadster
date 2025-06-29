use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::config::{CompletedAction, failure_action, retry_delay, success_action};
use crate::worker::job::{Job, JobMetadata};
use crate::worker::{EnqueueConfig, Worker, WorkerConfig};
use axum_core::extract::FromRef;
use builder::ProcessorBuilder;
use chrono::{DateTime, TimeDelta, Utc};
use itertools::Itertools;
use pgmq::PGMQueue;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PgProcessorError {
    /// The provided [`Worker`] was already registered. Contains the [`Worker::name`]
    /// of the provided worker.
    #[error("The provided `Worker` was already registered: `{0}`")]
    AlreadyRegistered(String),

    #[error("No queue configured for worker `{0}`.")]
    NoQueue(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Clone)]
#[non_exhaustive]
pub struct Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Arc<ProcessorInner<S>>,
}

#[non_exhaustive]
pub struct ProcessorInner<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    queues: BTreeSet<String>,
    workers: BTreeMap<String, WorkerWrapper<S>>,
    // cancellation_token: CancellationToken,
}

/*
How to implement periodic jobs? Maybe something like this:

1. Have a special "periodic" queue
2. On app/server launch, destroy/purge the "periodic" queue before re-registering all the periodic jobs

This might work, but it might need some experimentation to make sure it works wll with a lot of
app instances being deployed at once.
 */

impl<S> Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(inner: ProcessorInner<S>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn builder(state: &S) -> ProcessorBuilder<S> {
        ProcessorBuilder::new(state)
    }

    // pub fn cancellation_token(&self) -> CancellationToken {
    //     self.inner.cancellation_token.clone()
    // }

    pub async fn before_run(&self) -> RoadsterResult<()> {
        self.initialize_queues().await?;
        // remove_stale_periodic_jobs(&mut conn, &context, &self.registered_periodic_workers).await
        Ok(())
    }

    /// Ensures all of the workers' queues' tables exist in the Postgres database.
    pub async fn initialize_queues(&self) -> RoadsterResult<()> {
        let context = AppContext::from_ref(&self.inner.state);
        for queue in self.inner.queues.iter() {
            context.pgmq().create(queue).await?;
        }
        Ok(())
    }

    pub async fn run(self, cancellation_token: CancellationToken) {
        let mut join_set = JoinSet::new();

        let context = AppContext::from_ref(&self.inner.state);
        let worker_config = &context.config().service.worker.pg.custom;
        let dedicated_queues = &worker_config.common.queue_config;
        let shared_queues = worker_config
            .common
            .queues
            .as_ref()
            .unwrap_or(&self.inner.queues)
            .iter()
            .filter(|queue| !dedicated_queues.contains_key(*queue))
            .map(|queue| queue.to_owned())
            .collect_vec();

        if !shared_queues.is_empty() {
            for worker_num in 0..worker_config.common.num_workers {
                join_set.spawn(self.clone().process_queues(
                    cancellation_token.clone(),
                    worker_num,
                    shared_queues.clone(),
                ));
            }
        }

        for (queue, config) in dedicated_queues {
            for worker_num in 0..config.num_workers.unwrap_or_default() {
                join_set.spawn(self.clone().process_queues(
                    cancellation_token.clone(),
                    worker_num,
                    vec![queue.to_owned()],
                ));
            }
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
        worker_num: u32,
        queues: Vec<String>,
    ) {
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

        let pgmq = context.pgmq();
        loop {
            while let Some(mut queue) = queues.peek_mut() {
                if cancellation_token.is_cancelled() {
                    // todo: differentiate between shared and dedicated worker loops?
                    // todo: add total number of worker tasks to the event?
                    info!(worker_num, "Exiting processor worker loop");
                    return;
                }

                {
                    let diff = max(TimeDelta::zero(), queue.next_fetch - Utc::now());
                    let duration = diff.to_std().unwrap_or_else(|_| Duration::from_secs(0));
                    sleep(duration).await;
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
                        let delay = context
                            .config()
                            .service
                            .worker
                            .pg
                            .custom
                            .custom
                            .queue_fetch_config
                            .as_ref()
                            .and_then(|config| config.empty_delay);
                        queue.next_fetch = Utc::now() + delay.unwrap_or_default();
                        continue;
                    }
                    Err(err) => {
                        error!(
                            worker_num,
                            queue = queue.name,
                            "An error occurred while reading from pgmq: {err}"
                        );
                        let delay = context
                            .config()
                            .service
                            .worker
                            .pg
                            .custom
                            .custom
                            .queue_fetch_config
                            .as_ref()
                            .and_then(|config| config.error_delay);
                        queue.next_fetch = Utc::now() + delay.unwrap_or_default();
                        continue;
                    }
                };

                let job: Job = match serde_json::from_value(msg.message) {
                    Ok(job) => job,
                    Err(err) => {
                        error!(
                            msg_id = msg.msg_id,
                            read_count = msg.read_ct,
                            worker_num,
                            queue = queue.name,
                            "An error occurred while deserializing message from pgmq: {err}"
                        );
                        self.retry(
                            pgmq,
                            worker_num,
                            &queue,
                            None,
                            msg.msg_id,
                            msg.read_ct,
                            default_worker_config,
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
                        worker_num,
                        queue = queue.name,
                        worker_name = job.metadata.worker_name,
                        "Unable to handle job, worker not registered"
                    );
                    self.retry(
                        pgmq,
                        worker_num,
                        &queue,
                        Some(&job.metadata),
                        msg.msg_id,
                        msg.read_ct,
                        default_worker_config,
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
                        worker_num,
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
                        worker_num,
                        queue = queue.name,
                        worker_name = job.metadata.worker_name,
                        "An error occurred while handling a job: {err}"
                    );
                    self.retry(
                        pgmq,
                        worker_num,
                        &queue,
                        Some(&job.metadata),
                        msg.msg_id,
                        msg.read_ct,
                        default_worker_config,
                        Some(worker),
                    )
                    .await;
                } else {
                    let action = success_action(
                        default_worker_config.pg.as_ref(),
                        worker.worker_config.pg.as_ref(),
                    );
                    self.job_completed(
                        pgmq,
                        worker_num,
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

    #[instrument(skip_all)]
    async fn retry(
        &self,
        pgmq: &PGMQueue,
        worker_num: u32,
        queue: &QueueItem,
        job_metadata: Option<&JobMetadata>,
        msg_id: i64,
        read_count: i32,
        default_worker_config: &WorkerConfig,
        worker: Option<&WorkerWrapper<S>>,
    ) {
        if let Some(delay) = retry_delay(
            default_worker_config.retry_config.as_ref(),
            worker.and_then(|worker| worker.worker_config.retry_config.as_ref()),
            read_count,
        ) {
            // If the job can retry, update its view timeout by the calculated delay.
            self.update_job_view_timeout(
                pgmq,
                worker_num,
                queue,
                job_metadata,
                msg_id,
                read_count,
                delay,
            )
            .await;
        } else {
            // Otherwise, perform the failure action for the worker.
            let action = failure_action(
                default_worker_config.pg.as_ref(),
                worker.and_then(|worker| worker.worker_config.pg.as_ref()),
            );
            self.job_completed(
                pgmq,
                worker_num,
                queue,
                job_metadata,
                msg_id,
                read_count,
                action,
            )
            .await;
        }
    }

    #[instrument(skip_all)]
    async fn update_job_view_timeout(
        &self,
        pgmq: &PGMQueue,
        worker_num: u32,
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
                worker_num,
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
        worker_num: u32,
        queue: &QueueItem,
        job_metadata: Option<&JobMetadata>,
        msg_id: i64,
        read_count: i32,
        action: &CompletedAction,
    ) {
        debug!(
            msg_id,
            read_count,
            worker_num,
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
                worker_num,
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
        // Todo: without this `'static`, we're getting an internal compiler error
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
    mod queue_item {
        use crate::worker::backend::pg::processor::QueueItem;
        use chrono::Utc;
        use std::collections::BinaryHeap;
        use std::time::Duration;

        #[test]
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
}
