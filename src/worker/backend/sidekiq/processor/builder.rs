use crate::app::context::AppContext;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::worker::backend::shared_queues;
use crate::worker::backend::sidekiq::processor::{
    RegisterSidekiqFn, RegisterSidekiqMiddlewareFn, RegisterSidekiqPeriodicFn, SidekiqProcessor,
    SidekiqProcessorError, SidekiqProcessorInner, WorkerData,
};
use crate::worker::backend::sidekiq::roadster_worker::RoadsterWorker;
use crate::worker::job::Job;
use crate::worker::{PeriodicArgs, PeriodicArgsJson, Worker, WorkerWrapper};
use axum_core::extract::FromRef;
use itertools::Itertools;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use sidekiq::{Processor, ServerMiddleware};
use std::any::Any;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[non_exhaustive]
pub struct SidekiqProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) state: S,
    pub(crate) queues: BTreeSet<String>,
    pub(crate) workers: BTreeMap<String, Arc<WorkerData<S>>>,
    pub(crate) periodic_workers: HashMap<PeriodicArgsJson, Arc<WorkerData<S>>>,
    pub(crate) middleware: Vec<RegisterSidekiqMiddlewareFn>,
}

impl<S> SidekiqProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            state: state.clone(),
            queues: Default::default(),
            workers: Default::default(),
            periodic_workers: Default::default(),
            middleware: Default::default(),
        }
    }

    pub async fn build(self) -> RoadsterResult<SidekiqProcessor<S>> {
        let context = AppContext::from_ref(&self.state);

        let mut processor = if let Some(redis) = context.redis_fetch() {
            let config = &context.config().service.worker.sidekiq.custom.common;

            let num_workers = config.num_workers.to_usize().ok_or_else(|| {
                crate::error::other::OtherError::Message(format!(
                    "Unable to convert num_workers `{}` to usize",
                    context
                        .config()
                        .service
                        .worker
                        .sidekiq
                        .custom
                        .common
                        .num_workers
                ))
            })?;

            let processor_config = ::sidekiq::ProcessorConfig::default()
                .num_workers(num_workers)
                .balance_strategy(config.balance_strategy.clone().into());
            let processor_config = config.queue_config.iter().fold(
                processor_config,
                |processor_config, (queue, config)| {
                    processor_config.queue_config(queue.clone(), config.into())
                },
            );

            let shared_queues = self.shared_queues(context.config());
            let processor = ::sidekiq::Processor::new(redis.clone().inner, shared_queues)
                .with_config(processor_config);

            Some(processor)
        } else {
            None
        };

        if let Some(processor) = processor.as_mut() {
            for worker_data in self.workers.values() {
                (worker_data.register_sidekiq_fn)(
                    &self.state,
                    processor,
                    worker_data.worker_wrapper.clone(),
                );
            }

            for middleware in self.middleware {
                middleware(processor).await;
            }
        }

        Ok(SidekiqProcessor::new(SidekiqProcessorInner {
            state: self.state,
            processor: Mutex::new(processor),
            queues: self.queues,
            periodic_workers: self.periodic_workers,
        }))
    }

    fn shared_queues(&self, config: &AppConfig) -> Vec<String> {
        let worker_config = &config.service.worker.sidekiq.custom;
        shared_queues(
            &worker_config.common.queues,
            &self.queues,
            &worker_config.common.queue_config,
        )
        .map(|queue| queue.to_owned())
        .collect_vec()
    }

    pub fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(worker.name = name, "Registering Sidekiq worker");

        self.register_internal(worker, name, true)?;

        Ok(self)
    }

    pub fn register_periodic<W, Args, E>(
        mut self,
        worker: W,
        periodic_args: PeriodicArgs<Args>,
    ) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(worker.name = name, "Registering periodic PG worker");

        let worker_data = self.register_internal(worker, name.clone(), false)?;

        let periodic_args = PeriodicArgsJson::builder()
            .args(serde_json::to_value(periodic_args.args)?)
            .worker_name(name.clone())
            .schedule(periodic_args.schedule)
            .build();

        if self
            .periodic_workers
            .insert(periodic_args.clone(), worker_data)
            .is_some()
        {
            return Err(SidekiqProcessorError::AlreadyRegisteredPeriodic(
                periodic_args.worker_name,
                periodic_args.schedule.to_string(),
                periodic_args.args,
            )
            .into());
        }

        Ok(self)
    }

    pub async fn middleware<M>(mut self, middleware: M) -> RoadsterResult<Self>
    where
        M: ServerMiddleware + Send + Sync + 'static,
    {
        let register_sidekiq_middleware_fn: RegisterSidekiqMiddlewareFn =
            Box::new(move |processor| {
                Box::pin(async move {
                    processor.using(middleware).await;
                })
            });
        self.middleware.push(register_sidekiq_middleware_fn);
        Ok(self)
    }

    fn register_internal<W, Args, E>(
        &mut self,
        worker: W,
        name: String,
        err_on_duplicate: bool,
    ) -> RoadsterResult<Arc<WorkerData<S>>>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let context = AppContext::from_ref(&self.state);
        let enqueue_config = &context.config().service.worker.enqueue_config;
        let worker_enqueue_config = W::enqueue_config(&self.state);

        if let Some(registered_worker) = self.workers.get(&name) {
            return if registered_worker.worker_wrapper.inner.type_id != worker.type_id() {
                Err(SidekiqProcessorError::AlreadyRegisteredWithDifferentType(name).into())
            } else if err_on_duplicate {
                Err(SidekiqProcessorError::AlreadyRegistered(name).into())
            } else {
                // Already registered with the same type, no need to do anything
                Ok(registered_worker.clone())
            };
        }

        let queue = worker_enqueue_config
            .queue
            .as_ref()
            .or(enqueue_config.queue.as_ref());
        let queue = if let Some(queue) = queue {
            queue.to_owned()
        } else {
            error!(
                worker.name = W::name(),
                "Unable to register worker, no queue configured"
            );
            return Err(SidekiqProcessorError::NoQueue(W::name()).into());
        };
        self.queues.insert(queue.clone());

        let register_sidekiq_fn: RegisterSidekiqFn<S> = Box::new(
            move |state: &S, processor: &mut Processor, worker_wrapper: WorkerWrapper<S>| {
                let roadster_worker = RoadsterWorker::<S, W, Args, E>::new(state, worker_wrapper);
                processor.register(roadster_worker);
            },
        );

        let register_sidekiq_periodic_fn: RegisterSidekiqPeriodicFn<S> = Box::new(
            move |state: &S,
                  processor: &mut Processor,
                  worker_wrapper: WorkerWrapper<S>,
                  args: PeriodicArgsJson| {
                let queue = queue.clone();
                Box::pin(async move {
                    let roadster_worker =
                        RoadsterWorker::<S, W, Args, E>::new(state, worker_wrapper);
                    let mut job = Job::from(&args);
                    let id = job
                        .metadata
                        .periodic
                        .as_ref()
                        .map(|p| p.hash)
                        .map(|hash| hash.to_string());
                    let id = if let Some(id) = id {
                        id
                    } else {
                        warn!("Periodic job created without a hash/id");
                        Default::default()
                    };
                    job.metadata.id = id.clone();
                    let builder = ::sidekiq::periodic::builder(&args.schedule.to_string())?
                        .args(job)?
                        .queue(queue.clone());
                    builder.register(processor, roadster_worker).await?;
                    Ok(id)
                })
            },
        );

        let worker_data = Arc::new(WorkerData {
            worker_wrapper: WorkerWrapper::new(&self.state, worker, worker_enqueue_config)?,
            register_sidekiq_fn,
            register_sidekiq_periodic_fn,
        });
        self.workers.insert(name.clone(), worker_data.clone());

        Ok(worker_data)
    }
}
