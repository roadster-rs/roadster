use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::backend::pg::processor::{PgProcessor, PgProcessorError, PgProcessorInner};
use crate::worker::{PeriodicArgs, PeriodicArgsJson, Worker, WorkerWrapper};
use axum_core::extract::FromRef;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
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

    pub fn build(self) -> RoadsterResult<PgProcessor<S>> {
        Ok(PgProcessor::new(self.inner))
    }

    pub fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let name = W::name();
        info!(name, "Registering PG worker");

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
        info!(name, "Registering periodic PG worker");

        self.register_internal(worker, name.clone(), false)?;

        let periodic_args = PeriodicArgsJson::builder()
            .args(serde_json::to_value(periodic_args.args)?)
            .worker_name(name.clone())
            .schedule(periodic_args.schedule)
            .build();

        if let Some(replaced) = self.inner.periodic_workers.replace(periodic_args) {
            return Err(PgProcessorError::AlreadyRegisteredPeriodic(
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
            && err_on_duplicate
        {
            return Err(PgProcessorError::AlreadyRegistered(name).into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::worker::backend::pg::processor::builder::PgProcessorBuilder;
    use crate::worker::config::EnqueueConfig;
    use crate::worker::enqueue::Enqueuer;
    use crate::worker::{PeriodicArgs, Worker};
    use async_trait::async_trait;
    use axum_core::extract::FromRef;
    use cron::Schedule;
    use rstest::{fixture, rstest};
    use serde::{Deserialize, Serialize};
    use std::borrow::Borrow;
    use std::str::FromStr;
    use std::time::Duration;

    struct TestEnqueuer;
    #[async_trait]
    impl Enqueuer for TestEnqueuer {
        type Error = crate::error::Error;

        async fn enqueue<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: ArgsRef,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            todo!()
        }

        async fn enqueue_delayed<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: ArgsRef,
            _delay: Duration,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            todo!()
        }

        async fn enqueue_batch<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: &[ArgsRef],
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            todo!()
        }

        async fn enqueue_batch_delayed<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: &[ArgsRef],
            _delay: Duration,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            todo!()
        }
    }

    struct TestWorker;
    #[async_trait]
    impl Worker<AppContext, ()> for TestWorker {
        type Error = crate::error::Error;
        type Enqueuer = TestEnqueuer;

        fn enqueue_config(_state: &AppContext) -> EnqueueConfig {
            EnqueueConfig::builder().queue("default").build()
        }

        async fn handle(&self, _state: &AppContext, _args: ()) -> Result<(), Self::Error> {
            todo!()
        }
    }

    struct TestWorkerNoQueue;
    #[async_trait]
    impl Worker<AppContext, ()> for TestWorkerNoQueue {
        type Error = crate::error::Error;
        type Enqueuer = TestEnqueuer;

        async fn handle(&self, _state: &AppContext, _args: ()) -> Result<(), Self::Error> {
            todo!()
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

    mod periodic_args {
        use crate::worker::PeriodicArgsJson;
        use crate::worker::job::Job;
        use cron::Schedule;
        use insta::assert_json_snapshot;
        use rstest::{fixture, rstest};
        use std::hash::DefaultHasher;
        use std::hash::Hasher;
        use std::str::FromStr;

        #[fixture]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn periodic_args_json() -> PeriodicArgsJson {
            PeriodicArgsJson::builder()
                .worker_name("a".to_string())
                .schedule(Schedule::from_str("* * * * * *").unwrap())
                .args(serde_json::json!({"foo": "bar"}))
                .build()
        }

        // Todo: do we need any more tests for the args hash?
        // #[rstest]
        // #[cfg_attr(coverage_nightly, coverage(off))]
        // fn periodic_args_json_ord_name(periodic_args_json: PeriodicArgsJson) {
        //     let mut b = periodic_args_json.clone();
        //     b.worker_name = "b".to_string();
        //     assert!(periodic_args_json < b);
        // }
        //
        // #[rstest]
        // #[cfg_attr(coverage_nightly, coverage(off))]
        // fn periodic_args_json_ord_schedule(periodic_args_json: PeriodicArgsJson) {
        //     let mut b = periodic_args_json.clone();
        //     b.schedule = Schedule::from_str("*/10 * * * * *").unwrap();
        //     assert!(periodic_args_json < b);
        // }
        //
        // #[rstest]
        // #[cfg_attr(coverage_nightly, coverage(off))]
        // fn periodic_args_json_ord_args(periodic_args_json: PeriodicArgsJson) {
        //     let mut b = periodic_args_json.clone();
        //     b.args = serde_json::json!({"foo": "baz"});
        //     assert!(periodic_args_json < b);
        // }
        //
        // #[rstest]
        // #[cfg_attr(coverage_nightly, coverage(off))]
        // fn job_from_periodic_args(periodic_args_json: PeriodicArgsJson) {
        //     let job = Job::from(&periodic_args_json);
        //     assert_json_snapshot!(job);
        // }

        #[rstest]
        #[cfg_attr(coverage_nightly, coverage(off))]
        fn job_from_periodic_args_hash(periodic_args_json: PeriodicArgsJson) {
            let job = Job::from(&periodic_args_json);
            let mut hasher = DefaultHasher::new();
            crate::worker::job::periodic_hash(
                &mut hasher,
                &job.metadata.worker_name,
                &job.metadata.periodic.as_ref().unwrap().schedule,
                &job.args,
            );
            assert_eq!(hasher.finish(), job.metadata.periodic.unwrap().hash);
        }
    }
}
