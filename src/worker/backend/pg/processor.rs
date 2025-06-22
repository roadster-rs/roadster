use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::enqueue::queue_from_worker;
use crate::worker::job::Job;
use crate::worker::{EnqueueConfig, Worker, WorkerConfig};
use axum_core::extract::FromRef;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use sidekiq::redis_rs::ExpireOption::NONE;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use typed_builder::TypedBuilder;

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
        // let mut join_set = JoinSet::new();
        // let cancellation_token = self.inner.cancellation_token.clone();
        // join_set.spawn(Box::pin(async move {
        //     cancellation_token.cancelled().await;
        // }));
        // let token = sidekiq_cancel_token.clone();
        // join_set.spawn(Box::pin(async move {
        //     token.cancelled().await;
        // }));
        // join_set.spawn(processor.run());
        //
        // while let Some(result) = join_set.join_next().await {
        //     cancellation_token.cancel();
        //     if let Err(join_err) = result {
        //         error!(
        //             "An error occurred when trying to join on one of the app's tasks. Error: {join_err}"
        //         );
        //     }
        // }

        // let mut join_set = JoinSet::new();

        // self.inner.workers

        let mut join_set = JoinSet::new();

        // for i in 0..context.config().service.worker.pg.custom.common.num_workers {
        //     join_set.spawn(self.process_queues())
        // }

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
        let context = AppContext::from_ref(&self.inner.state);
        loop {
            for queue in queues.iter() {
                if self.inner.cancellation_token.is_cancelled() {
                    info!(worker_num, "Exiting processor worker loop");
                    return;
                }

                let msg = match context.pgmq().read::<String>(queue, None).await {
                    Ok(Some(msg)) => msg,
                    Ok(None) => continue,
                    Err(err) => {
                        error!(
                            worker_num,
                            queue, "An error occurred while reading from pgmq: {err}"
                        );
                        continue;
                    }
                };

                let job: Job = match serde_json::from_str(&msg.message) {
                    Ok(job) => job,
                    Err(err) => {
                        error!(
                            worker_num,
                            queue, "An error occurred while deserializing message from pgmq: {err}"
                        );
                        continue;
                    }
                };

                let worker = if let Some(worker) = self.inner.workers.get(job.metadata.worker_name)
                {
                    worker
                } else {
                    error!(
                        worker_num,
                        queue,
                        worker_name = job.metadata.worker_name,
                        "Unable to handle job, worker not registered"
                    );
                    continue;
                };

                // Todo: error handling
                worker.handle(&self.inner.state, &job.args).await.unwrap();
            }
        }
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
