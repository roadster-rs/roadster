use crate::app::context::AppContext;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::worker::backend::shared_queues;
use crate::worker::backend::sidekiq::processor::{
    RegisterSidekiqFn, RegisterSidekiqMiddlewareFn, RegisterSidekiqPeriodicFn, SidekiqProcessor,
    SidekiqProcessorError, SidekiqProcessorInner, WorkerData,
};
use crate::worker::backend::sidekiq::roadster_worker::RoadsterWorker;
use crate::worker::job::{Job, JobMetadata, periodic_hash};
use crate::worker::{PeriodicArgs, PeriodicArgsJson, Worker, WorkerWrapper};
use axum_core::extract::FromRef;
use itertools::Itertools;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use sidekiq::{Processor, ServerMiddleware};
use std::any::Any;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info};

// Todo: The `SidekiqProcessorBuilder` and the `PgProcessorBuilder` have a lot of similar code, can
//  we consolidate some of it?
#[non_exhaustive]
pub struct SidekiqProcessorBuilder<S>
where
    S: 'static + Send + Sync + Clone,
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
    S: 'static + Send + Sync + Clone,
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
        E: 'static + Send + Sync + std::error::Error,
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
        E: 'static + Send + Sync + std::error::Error,
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
        M: 'static + Send + Sync + ServerMiddleware,
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
        E: 'static + Send + Sync + std::error::Error,
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

        let register_sidekiq_periodic_fn: RegisterSidekiqPeriodicFn<S> =
            Box::new(
                move |state: &S,
                      processor: &mut Processor,
                      worker_wrapper: WorkerWrapper<S>,
                      args: PeriodicArgsJson| {
                    let queue = queue.clone();
                    Box::pin(async move {
                        use sidekiq::Worker as SidekiqWorker;

                        /*
                        We need a deterministic job id for periodic jobs in order to avoid creating
                        duplicate jobs in Redis. This is because Redis dedupes on the entire serialized
                        job, so having non-deterministic ID (e.g., a UUID) would result in duplicate
                        entries being created in Redis. So, for periodic jobs, we use the periodic hash
                        as the job ID.
                         */
                        let mut hash = DefaultHasher::new();
                        periodic_hash(&mut hash, &args.worker_name, &args.schedule, &args.args);
                        let hash = hash.finish();

                        let job = Job::builder()
                            .args(args.args)
                            .metadata(
                                JobMetadata::builder()
                                    .id(hash)
                                    .worker_name(args.worker_name)
                                    .build(),
                            )
                            .build();

                        let builder = ::sidekiq::periodic::builder(&args.schedule.to_string())?
                            .args(job)?
                            .queue(queue.clone());

                        let json = serde_json::to_string(
                            &builder
                                .into_periodic_job(RoadsterWorker::<S, W, Args, E>::class_name())?,
                        )?;

                        let roadster_worker =
                            RoadsterWorker::<S, W, Args, E>::new(state, worker_wrapper);
                        builder.register(processor, roadster_worker).await?;

                        Ok(json)
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

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::worker::backend::sidekiq::processor::builder::SidekiqProcessorBuilder;
    use crate::worker::enqueue::test::TestEnqueuer;
    use crate::worker::test::TestWorker;
    use crate::worker::{PeriodicArgs, Worker};
    use async_trait::async_trait;
    use cron::Schedule;
    use rstest::{fixture, rstest};
    use std::str::FromStr;

    struct TestWorkerNoQueue;
    #[async_trait]
    impl Worker<AppContext, ()> for TestWorkerNoQueue {
        type Error = crate::error::Error;
        type Enqueuer = TestEnqueuer;

        async fn handle(&self, _state: &AppContext, _args: ()) -> Result<(), Self::Error> {
            unimplemented!()
        }
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn context() -> AppContext {
        AppContext::test(None, None, None).unwrap()
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder(context: AppContext) -> SidekiqProcessorBuilder<AppContext> {
        SidekiqProcessorBuilder::new(&context)
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register(builder: SidekiqProcessorBuilder<AppContext>) {
        builder.register(TestWorker).unwrap();
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_duplicate(builder: SidekiqProcessorBuilder<AppContext>) {
        let result = builder.register(TestWorker).unwrap().register(TestWorker);
        assert!(result.is_err());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_no_queue(builder: SidekiqProcessorBuilder<AppContext>) {
        let result = builder.register(TestWorkerNoQueue);
        assert!(result.is_err());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_periodic(builder: SidekiqProcessorBuilder<AppContext>) {
        builder
            .register_periodic(
                TestWorker,
                PeriodicArgs::builder()
                    .args(())
                    .schedule(Schedule::from_str("* * * * * *").unwrap())
                    .build(),
            )
            .unwrap();
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_periodic_duplicate(builder: SidekiqProcessorBuilder<AppContext>) {
        let result = builder
            .register_periodic(
                TestWorker,
                PeriodicArgs::builder()
                    .args(())
                    .schedule(Schedule::from_str("* * * * * *").unwrap())
                    .build(),
            )
            .unwrap()
            .register_periodic(
                TestWorker,
                PeriodicArgs::builder()
                    .args(())
                    .schedule(Schedule::from_str("* * * * * *").unwrap())
                    .build(),
            );
        assert!(result.is_err());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_periodic_same_worker(builder: SidekiqProcessorBuilder<AppContext>) {
        let result = builder
            .register_periodic(
                TestWorker,
                PeriodicArgs::builder()
                    .args(())
                    .schedule(Schedule::from_str("* * * * * *").unwrap())
                    .build(),
            )
            .unwrap()
            .register_periodic(
                TestWorker,
                PeriodicArgs::builder()
                    .args(())
                    .schedule(Schedule::from_str("*/10 * * * * *").unwrap())
                    .build(),
            );
        assert!(result.is_ok());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_periodic_no_queue(builder: SidekiqProcessorBuilder<AppContext>) {
        let result = builder.register_periodic(
            TestWorkerNoQueue,
            PeriodicArgs::builder()
                .args(())
                .schedule(Schedule::from_str("* * * * * *").unwrap())
                .build(),
        );
        assert!(result.is_err());
    }
}
