use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::backend::pg::processor::{PgProcessor, PgProcessorError, PgProcessorInner};
use crate::worker::{PeriodicArgs, PeriodicArgsJson, Worker, WorkerWrapper};
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::any::Any;
use tracing::{error, info};

#[non_exhaustive]
pub struct PgProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) inner: PgProcessorInner<S>,
}

impl<S> PgProcessorBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub(crate) fn new(state: &S) -> Self {
        Self {
            inner: PgProcessorInner {
                state: state.clone(),
                queues: Default::default(),
                workers: Default::default(),
                periodic_workers: Default::default(),
            },
        }
    }

    pub async fn build(self) -> RoadsterResult<PgProcessor<S>> {
        Ok(PgProcessor::new(self.inner))
    }

    pub fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(worker.name = name, "Registering PG worker");

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
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(worker.name = name, "Registering periodic PG worker");

        self.register_internal(worker, name.clone(), false)?;

        let periodic_args = PeriodicArgsJson::builder()
            .args(serde_json::to_value(periodic_args.args)?)
            .worker_name(name.clone())
            .schedule(periodic_args.schedule)
            .build();

        if let Some(replaced) = self.inner.periodic_workers.replace(periodic_args) {
            return Err(PgProcessorError::AlreadyRegisteredPeriodic(
                replaced.worker_name,
                replaced.schedule.to_string(),
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
        err_on_duplicate: bool,
    ) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let context = AppContext::from_ref(&self.inner.state);
        let enqueue_config = &context.config().service.worker.enqueue_config;
        let worker_enqueue_config = W::enqueue_config(&self.inner.state);

        if let Some(registered_worker) = self.inner.workers.get(&name) {
            return if registered_worker.inner.type_id != worker.type_id() {
                Err(PgProcessorError::AlreadyRegisteredWithDifferentType(name).into())
            } else if err_on_duplicate {
                Err(PgProcessorError::AlreadyRegistered(name).into())
            } else {
                // Already registered with the same type, no need to do anything
                Ok(())
            };
        }

        let queue = worker_enqueue_config
            .queue
            .as_ref()
            .or(enqueue_config.queue.as_ref());
        let queue = if let Some(queue) = queue {
            queue
        } else {
            error!(
                worker.name = W::name(),
                "Unable to register worker, no queue configured"
            );
            return Err(PgProcessorError::NoQueue(W::name()).into());
        };
        self.inner.queues.insert(queue.to_owned());

        self.inner.workers.insert(
            name.clone(),
            WorkerWrapper::new(&self.inner.state, worker, worker_enqueue_config)?,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::worker::backend::pg::processor::builder::PgProcessorBuilder;
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
    fn builder(context: AppContext) -> PgProcessorBuilder<AppContext> {
        PgProcessorBuilder::new(&context)
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register(builder: PgProcessorBuilder<AppContext>) {
        builder.register(TestWorker).unwrap();
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_duplicate(builder: PgProcessorBuilder<AppContext>) {
        let result = builder.register(TestWorker).unwrap().register(TestWorker);
        assert!(result.is_err());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_no_queue(builder: PgProcessorBuilder<AppContext>) {
        let result = builder.register(TestWorkerNoQueue);
        assert!(result.is_err());
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn builder_register_periodic(builder: PgProcessorBuilder<AppContext>) {
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
    fn builder_register_periodic_duplicate(builder: PgProcessorBuilder<AppContext>) {
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
    fn builder_register_periodic_same_worker(builder: PgProcessorBuilder<AppContext>) {
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
    fn builder_register_periodic_no_queue(builder: PgProcessorBuilder<AppContext>) {
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
