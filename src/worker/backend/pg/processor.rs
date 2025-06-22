use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::job::Job;
use crate::worker::{EnqueueConfig, Worker, WorkerConfig};
use axum_core::extract::FromRef;
use chrono::{DateTime, OutOfRangeError, TimeDelta, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::cmp::{Ordering, max};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::task::JoinSet;
use tokio::time::{sleep, sleep_until};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

const DEFAULT_VIEW_TIMEOUT: Duration = Duration::from_secs(60 * 10);

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
    cancellation_token: CancellationToken,
}

#[non_exhaustive]
pub struct ProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub inner: ProcessorInner<S>,
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
    fn new(inner: ProcessorInner<S>) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn builder(state: &S) -> ProcessorBuilder<S> {
        ProcessorBuilder::new(state)
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.inner.cancellation_token.clone()
    }

    /// Ensures all of the workers' queues' tables exist in the Postgres database.
    pub async fn initialize_queues(&self) -> RoadsterResult<()> {
        let context = AppContext::from_ref(&self.inner.state);
        for queue in self.inner.queues.iter() {
            context.pgmq().create(queue).await?;
        }
        Ok(())
    }

    pub async fn run(self) -> RoadsterResult<()> {
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
                join_set.spawn(
                    self.clone()
                        .process_queues(worker_num, shared_queues.clone()),
                );
            }
        }

        for (queue, config) in dedicated_queues {
            for worker_num in 0..config.num_workers.unwrap_or_default() {
                join_set.spawn(
                    self.clone()
                        .process_queues(worker_num, vec![queue.to_owned()]),
                );
            }
        }

        while let Some(result) = join_set.join_next().await {
            // Once any of the tasks finish, cancel the cancellation token to ensure
            // the processor and the app shut down gracefully.
            self.inner.cancellation_token.cancel();
            if let Err(join_err) = result {
                error!(
                    "An error occurred when trying to join on one of the processor's workers. Error: {join_err}"
                );
            }
        }

        Ok(())
    }

    async fn process_queues(self, worker_num: u32, queues: Vec<String>) {
        let mut queues: BinaryHeap<QueueItem> = queues
            .into_iter()
            .map(|name| QueueItem {
                name,
                next_fetch: Utc::now(),
            })
            .collect();

        let context = AppContext::from_ref(&self.inner.state);
        let default_max_duration = context.config().service.worker.worker_config.max_duration;
        let default_view_timeout = default_max_duration
            .as_ref()
            .map(|duration| duration.as_secs())
            .map(|duration| {
                // Todo: is there a utility/crate to do this safely?
                if duration > (i32::MAX as u64) {
                    i32::MAX
                } else {
                    duration as i32
                }
            });

        loop {
            // todo: confirm that updating the `peek_mut` object updates the binary heap
            while let Some(mut queue) = queues.peek_mut() {
                if self.inner.cancellation_token.is_cancelled() {
                    info!(worker_num, "Exiting processor worker loop");
                    return;
                }

                let diff = max(TimeDelta::zero(), queue.next_fetch - Utc::now());
                let duration = diff.to_std().unwrap_or_else(|_| Duration::from_secs(0));
                // Todo: do we need this check, or is `sleep` smart enough to do nothing if the duration is zero
                // Todo: maybe don't sleep if duration is less than X milliseconds
                if !duration.is_zero() {
                    sleep(duration).await;
                }

                // Todo: We can't deserialize to a string, so we'll probably want to use serde_json::Value
                //  for the job args instead of a json string
                let msg = match context
                    .pgmq()
                    .read::<String>(&queue.name, default_view_timeout)
                    .await
                {
                    Ok(Some(msg)) => msg,
                    Ok(None) => {
                        // Todo: make this configurable
                        // Todo: consolidate the `next_fetch` update logic so we don't need to
                        //  remember to do it before every `continue`
                        queue.next_fetch = Utc::now() + Duration::from_secs(10);
                        continue;
                    }
                    Err(err) => {
                        error!(
                            worker_num,
                            queue = queue.name,
                            "An error occurred while reading from pgmq: {err}"
                        );
                        // Todo: make this configurable
                        queue.next_fetch = Utc::now() + Duration::from_secs(10);
                        continue;
                    }
                };

                let job: Job = match serde_json::from_str(&msg.message) {
                    Ok(job) => job,
                    Err(err) => {
                        error!(
                            msg_id = msg.msg_id,
                            read_count = msg.read_ct,
                            worker_num,
                            queue = queue.name,
                            "An error occurred while deserializing message from pgmq, archiving: {err}"
                        );
                        if let Err(err) = context.pgmq().archive(&queue.name, msg.msg_id).await {
                            error!(
                                msg_id = msg.msg_id,
                                read_count = msg.read_ct,
                                worker_num,
                                queue = queue.name,
                                "An error occurred while archiving message: {err}"
                            );
                        }
                        // Todo: make this configurable
                        queue.next_fetch = Utc::now() + Duration::from_secs(10);
                        continue;
                    }
                };

                let worker = if let Some(worker) = self.inner.workers.get(job.metadata.worker_name)
                {
                    worker
                } else {
                    // Todo: set the vt of the job with exponential backoff
                    error!(
                        msg_id = msg.msg_id,
                        read_count = msg.read_ct,
                        worker_num,
                        queue = queue.name,
                        worker_name = job.metadata.worker_name,
                        "Unable to handle job, worker not registered"
                    );
                    // Todo: make this configurable
                    queue.next_fetch = Utc::now() + Duration::from_secs(10);
                    continue;
                };

                if let Some(duration) = default_max_duration
                    .or_else(|| context.config().service.worker.worker_config.max_duration)
                {
                    if let Err(err) = context
                        .pgmq()
                        .set_vt::<serde_json::Value>(&queue.name, msg.msg_id, Utc::now() + duration)
                        .await
                    {
                        error!(
                            msg_id = msg.msg_id,
                            read_count = msg.read_ct,
                            worker_num,
                            queue = queue.name,
                            worker_name = job.metadata.worker_name,
                            "An error occurred while updating the view timeout of a job: {err}"
                        );
                    }
                }

                // Todo: error handling, here and for other "worker" errors above
                worker.handle(&self.inner.state, &job.args).await.unwrap();

                // On success, update the next_fetch to `now`
                // Todo: make this configurable
                queue.next_fetch = Utc::now();
            }
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
        // todo: confirm that this create a min heaps
        other.next_fetch.cmp(&self.next_fetch)
    }
}

impl<S> ProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn new(state: &S) -> Self {
        Self {
            inner: ProcessorInner {
                state: state.clone(),
                queues: Default::default(),
                workers: Default::default(),
                cancellation_token: Default::default(),
            },
        }
    }

    pub fn build(self) -> Processor<S> {
        Processor::new(self.inner)
    }

    pub async fn register<W, Args, E>(&mut self, worker: W) -> RoadsterResult<&mut Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering PG worker");

        let context = AppContext::from_ref(&self.inner.state);
        let enqueue_config = &context.config().service.worker.enqueue_config;
        let worker_enqueue_config = W::enqueue_config(&self.inner.state);

        let queue = if let Some(queue) = worker_enqueue_config.queue.as_ref() {
            queue
        } else if let Some(queue) = enqueue_config.queue.as_ref() {
            queue
        } else {
            error!(
                worker_name = W::name(),
                "Unable to register worker, no queue configured"
            );
            return Err(PgProcessorError::NoQueue(W::name()).into());
        };
        self.inner.queues.insert(queue.to_owned());

        if self
            .inner
            .workers
            .insert(
                name.clone(),
                WorkerWrapper::new(&self.inner.state, worker, worker_enqueue_config)?,
            )
            .is_some()
        {
            return Err(PgProcessorError::AlreadyRegistered(name).into());
        }
        Ok(self)
    }
}

type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S, &'a str) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

struct WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
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
            enqueue_config,
            worker_config: worker.worker_config(state),
            worker_fn: Box::new(move |state: &S, args: &str| {
                let worker = worker.clone();
                Box::pin(async move {
                    let args: Args = serde_json::from_str(args)
                        .map_err(crate::error::worker::DequeueError::Serde)?;

                    match worker.clone().handle(state, &args).await {
                        Ok(_) => Ok(()),
                        Err(err) => Err(crate::error::Error::from(
                            crate::error::worker::WorkerError::Handle(W::name(), Box::new(err)),
                        )),
                    }
                })
            }),
        })
    }

    async fn handle(&self, state: &S, args: &str) -> RoadsterResult<()> {
        // todo: timeouts, etc
        (self.worker_fn)(state, args).await?;
        Ok(())
    }
}
