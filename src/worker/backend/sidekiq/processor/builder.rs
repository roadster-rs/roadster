use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::backend::sidekiq::processor::{
    SidekiqProcessor, SidekiqProcessorError, SidekiqProcessorInner,
};
use crate::worker::backend::sidekiq::roadster_worker::RoadsterWorker;
use crate::worker::{PeriodicArgs, PeriodicArgsJson, RegisterSidekiqFn, Worker, WorkerWrapper};
use axum_core::extract::FromRef;
use itertools::Itertools;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use sidekiq::Processor;
use std::collections::{BTreeMap, BTreeSet};
use tracing::{error, info};

#[non_exhaustive]
pub struct SidekiqProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) state: S,
    pub(crate) processor: Option<::sidekiq::Processor>,
    pub(crate) queues: BTreeSet<String>,
    pub(crate) workers: BTreeMap<String, (WorkerWrapper<S>, RegisterSidekiqFn<S>)>,
    pub(crate) periodic_workers: BTreeSet<PeriodicArgsJson>,
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
            processor: Default::default(),
            workers: Default::default(),
            periodic_workers: Default::default(),
        }
    }

    pub fn build(mut self) -> RoadsterResult<SidekiqProcessor<S>> {
        let context = AppContext::from_ref(&self.state);

        let mut processor = if let Some(processor) = self.processor {
            Some(processor)
        } else if let Some(redis) = context.redis_fetch() {
            let config = &context.config().service.worker.sidekiq.custom.common;

            let dedicated_queues = &config.queue_config;

            let shared_queues = config
                .queues
                .clone()
                .unwrap_or_else(|| self.queues.clone())
                .iter()
                .filter(|queue| dedicated_queues.contains_key(*queue))
                .map(|s| s.to_owned())
                .collect_vec();

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

            let processor = ::sidekiq::Processor::new(redis.clone().inner, shared_queues)
                .with_config(processor_config);

            Some(processor)
        } else {
            None
        };

        if let Some(processor) = processor.as_mut() {
            for (worker, register_fn) in self.workers.into_values() {
                register_fn(&self.state, processor, worker);
            }
        }

        Ok(SidekiqProcessor::new(SidekiqProcessorInner {
            state: self.state,
            processor,
            queues: self.queues,
            periodic_workers: self.periodic_workers,
        }))
    }

    pub fn with_processor(mut self, processor: ::sidekiq::Processor) -> Self {
        self.processor = Some(processor);
        self
    }

    pub fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering Sidekiq worker");

        self.register_internal(worker, name, false)?;

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
        info!(name, "Registering periodic PG worker");

        self.register_internal(worker, name.clone(), true)?;

        let periodic_args = PeriodicArgsJson::builder()
            .args(serde_json::to_value(periodic_args.args)?)
            .worker_name(name.clone())
            .schedule(periodic_args.schedule)
            .build();

        if let Some(replaced) = self.periodic_workers.replace(periodic_args) {
            return Err(SidekiqProcessorError::AlreadyRegisteredPeriodic(
                replaced.worker_name,
                replaced.schedule,
                replaced.args,
            )
            .into());
        }

        Ok(self)
    }

    fn register_internal<W, Args, E>(
        &mut self,
        worker: W,
        name: String,
        skip_duplicate: bool,
    ) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: 'static + Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let context = AppContext::from_ref(&self.state);
        let enqueue_config = &context.config().service.worker.enqueue_config;
        let worker_enqueue_config = W::enqueue_config(&self.state);

        let queue = worker_enqueue_config
            .queue
            .as_ref()
            .or(enqueue_config.queue.as_ref());
        let queue = if let Some(queue) = queue {
            queue
        } else {
            error!(
                worker_name = W::name(),
                "Unable to register worker, no queue configured"
            );
            return Err(SidekiqProcessorError::NoQueue(W::name()).into());
        };
        self.queues.insert(queue.to_owned());

        // Todo: impl something similar for periodic jobs?
        let register_sidekiq = Box::new(
            move |state: &S, processor: &mut Processor, worker_wrapper: WorkerWrapper<S>| {
                let roadster_worker = RoadsterWorker::<S, W, Args, E>::new(state, worker_wrapper);
                processor.register(roadster_worker);
            },
        );

        if self
            .workers
            .insert(
                name.clone(),
                (
                    WorkerWrapper::new(&self.state, worker, worker_enqueue_config)?,
                    register_sidekiq,
                ),
            )
            .is_some()
            && !skip_duplicate
        {
            return Err(SidekiqProcessorError::AlreadyRegistered(name).into());
        }

        Ok(())
    }
}
