use crate::app::context::AppContext;
use crate::app::App;
use crate::config::service::worker::sidekiq::StaleCleanUpBehavior;
use crate::error::RoadsterResult;
use crate::service::worker::sidekiq::app_worker::{AppWorker, AppWorkerConfig};
#[cfg_attr(test, mockall_double::double)]
use crate::service::worker::sidekiq::processor_wrapper::ProcessorWrapper;
use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
use crate::service::worker::sidekiq::service::{enabled, SidekiqWorkerService, NAME};
use crate::service::AppServiceBuilder;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use itertools::Itertools;
use num_traits::ToPrimitive;
use serde::Serialize;
use sidekiq::{periodic, ProcessorConfig, ServerMiddleware, Worker};
use std::collections::HashSet;
use tracing::{debug, info};

pub(crate) const PERIODIC_KEY: &str = "periodic";

pub struct SidekiqWorkerServiceBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: BuilderState<S>,
}

#[allow(clippy::large_enum_variant)]
enum BuilderState<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    Enabled {
        processor: ProcessorWrapper,
        state: S,
        registered_workers: HashSet<String>,
        registered_periodic_workers: HashSet<String>,
    },
    Disabled,
}

#[async_trait]
impl<A, S> AppServiceBuilder<A, S, SidekiqWorkerService> for SidekiqWorkerServiceBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn name(&self) -> String {
        NAME.to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        match self.state {
            BuilderState::Enabled { .. } => enabled(&AppContext::from_ref(state)),
            BuilderState::Disabled => false,
        }
    }

    async fn build(self, _state: &S) -> RoadsterResult<SidekiqWorkerService> {
        let service = match self.state {
            BuilderState::Enabled {
                processor,
                registered_periodic_workers,
                ..
            } => SidekiqWorkerService {
                registered_periodic_workers,
                processor: processor.into_sidekiq_processor(),
            },
            BuilderState::Disabled => {
                return Err(anyhow!(
                    "This builder is not enabled; it's build method should not have been called."
                )
                .into());
            }
        };

        Ok(service)
    }
}

impl<S> SidekiqWorkerServiceBuilder<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub async fn with_processor(state: &S, processor: sidekiq::Processor) -> RoadsterResult<Self> {
        Self::new(state.clone(), Some(ProcessorWrapper::new(processor))).await
    }

    pub async fn with_default_processor(
        state: &S,
        worker_queues: Option<Vec<String>>,
    ) -> RoadsterResult<Self> {
        let context = AppContext::from_ref(state);
        let processor = if !enabled(&context) {
            debug!("Sidekiq service not enabled, not creating the Sidekiq processor");
            None
        } else if let Some(redis_fetch) = context.redis_fetch() {
            Self::auto_clean_periodic(&context).await?;
            let queues = context
                .config()
                .service
                .sidekiq
                .custom
                .queues
                .clone()
                .into_iter()
                .chain(worker_queues.unwrap_or_default())
                .collect_vec();
            info!(
                "Creating Sidekiq.rs (rusty-sidekiq) processor with {} queues",
                queues.len()
            );
            debug!("Sidekiq.rs queues: {queues:?}");
            let processor = {
                let config = context.config().service.sidekiq.custom.clone();
                let num_workers = config.num_workers.to_usize().ok_or_else(|| {
                    anyhow!(
                        "Unable to convert num_workers `{}` to usize",
                        context.config().service.sidekiq.custom.num_workers
                    )
                })?;
                let processor_config: ProcessorConfig = Default::default();
                let processor_config = processor_config
                    .num_workers(num_workers)
                    .balance_strategy(config.balance_strategy.into());

                let processor_config = context
                    .config()
                    .service
                    .sidekiq
                    .custom
                    .queue_config
                    .iter()
                    .fold(processor_config, |processor_config, (queue, config)| {
                        processor_config.queue_config(queue.clone(), config.into())
                    });

                let processor = sidekiq::Processor::new(redis_fetch.inner.clone(), queues.clone())
                    .with_config(processor_config);
                ProcessorWrapper::new(processor)
            };

            Some(processor)
        } else {
            debug!(
                "No 'redis fetch' connection pool configured, not creating the Sidekiq processor"
            );
            None
        };

        Self::new(state.clone(), processor).await
    }

    async fn new(state: S, processor: Option<ProcessorWrapper>) -> RoadsterResult<Self> {
        let context = AppContext::from_ref(&state);
        let processor = if enabled(&context) { processor } else { None };

        let state = if let Some(processor) = processor {
            BuilderState::Enabled {
                processor,
                state,
                registered_workers: Default::default(),
                registered_periodic_workers: Default::default(),
            }
        } else {
            BuilderState::Disabled
        };

        Ok(Self { state })
    }

    async fn auto_clean_periodic(context: &AppContext) -> RoadsterResult<()> {
        if context
            .config()
            .service
            .sidekiq
            .custom
            .periodic
            .stale_cleanup
            == StaleCleanUpBehavior::AutoCleanAll
        {
            // Periodic jobs are not removed automatically. Remove any periodic jobs that were
            // previously added. They should be re-added by `App::worker`.
            info!("Auto-cleaning periodic jobs");
            periodic::destroy_all(context.redis_enqueue().inner.clone()).await?;
        }

        Ok(())
    }

    /// Remove previously-registered periodic jobs from Sidekiq/Redis. This should be called
    /// before registering any new periodic jobs. If this method is called after a periodic job is
    /// registered, it will return an error.
    ///
    /// Periodic jobs can also be cleaned up automatically by setting the
    /// [service.sidekiq.periodic.stale-cleanup][crate::config::service::worker::sidekiq::StaleCleanUpBehavior]
    /// to `auto-clean-all` or `auto-clean-stale`.
    pub async fn clean_up_periodic_jobs(self) -> RoadsterResult<Self> {
        if let BuilderState::Enabled {
            registered_periodic_workers,
            state: context,
            ..
        } = &self.state
        {
            if !registered_periodic_workers.is_empty() {
                return Err(anyhow!("Can only clean up previous periodic jobs if no periodic jobs have been registered yet.").into());
            }
            let context = AppContext::from_ref(context);
            periodic::destroy_all(context.redis_enqueue().inner.clone()).await?;
        }

        Ok(self)
    }

    /// Register a [`Worker`] to handle Sidekiq.rs jobs.
    ///
    /// The worker will be wrapped by internal logic which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs. Note that this internal logic my not
    /// use the values of [`Worker::disable_argument_coercion`] and [`Worker::max_retries`] -- if
    /// the respective fields are set in the app's [`AppWorkerConfig`], that will be used first.
    /// It's recommended to use [`Self::register_worker_with_config`] to override the
    /// fields from the app's config as needed instead.
    pub fn register_worker<Args, W>(self, worker: W) -> RoadsterResult<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: Worker<Args> + 'static,
    {
        self.register_worker_inner(worker, None)
    }

    /// Register a [`Worker`] to handle Sidekiq.rs jobs.
    ///
    /// The worker will be wrapped by internal logic which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs. Note that this internal logic my not
    /// use the values of [`Worker::disable_argument_coercion`] and [`Worker::max_retries`] -- if
    /// the respective fields are set in the app's [`AppWorkerConfig`], that will be used first.
    /// It's recommended to use this method to override the fields from the app's config as needed
    /// instead.
    pub fn register_worker_with_config<Args, W>(
        self,
        worker: W,
        config: AppWorkerConfig,
    ) -> RoadsterResult<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: Worker<Args> + 'static,
    {
        self.register_worker_inner(worker, Some(config))
    }

    fn register_worker_inner<Args, W>(
        mut self,
        worker: W,
        config: Option<AppWorkerConfig>,
    ) -> RoadsterResult<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: Worker<Args> + 'static,
    {
        if let BuilderState::Enabled {
            processor,
            registered_workers,
            state,
            ..
        } = &mut self.state
        {
            let context = AppContext::from_ref(state);
            let class_name = W::class_name();
            debug!(worker = %class_name, "Registering worker");
            if !registered_workers.insert(class_name.clone()) {
                return Err(anyhow!("Worker `{class_name}` was already registered").into());
            }
            let roadster_worker = RoadsterWorker::new(&context, worker, config);
            processor.register(roadster_worker);
        }

        Ok(self)
    }

    /// Register a periodic [`Worker`] that will run with the provided args. The cadence
    /// of the periodic worker, the worker's queue name, and other attributes are specified using
    /// the [builder][periodic::Builder]. However, to help ensure type-safety the args are provided
    /// to this method instead of the [builder][periodic::Builder].
    ///
    /// The worker will be wrapped by internal logic which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs. Note that this internal logic my not
    /// use the values of [`Worker::disable_argument_coercion`] and [`Worker::max_retries`] -- if
    /// the respective fields are set in the app's [`AppWorkerConfig`], that will be used first.
    /// It's recommended to use [`Self::register_periodic_worker_with_config`] to override the
    /// fields from the app's config as needed instead.
    pub async fn register_periodic_worker<Args, W>(
        self,
        builder: periodic::Builder,
        worker: W,
        args: Args,
    ) -> RoadsterResult<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<S, Args> + 'static,
    {
        self.register_periodic_worker_inner(builder, worker, args, None)
            .await
    }

    /// Register a periodic [`Worker`] that will run with the provided args. The cadence
    /// of the periodic worker, the worker's queue name, and other attributes are specified using
    /// the [builder][periodic::Builder]. However, to help ensure type-safety the args are provided
    /// to this method instead of the [builder][periodic::Builder].
    ///
    /// The worker will be wrapped by internal logic which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs. Note that this internal logic my not
    /// use the values of [`Worker::disable_argument_coercion`] and [`Worker::max_retries`] -- if
    /// the respective fields are set in the app's [`AppWorkerConfig`], that will be used first.
    /// It's recommended to use this method to override the fields from the app's config as needed
    /// instead.
    pub async fn register_periodic_worker_with_config<Args, W>(
        self,
        builder: periodic::Builder,
        worker: W,
        args: Args,
        config: AppWorkerConfig,
    ) -> RoadsterResult<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<S, Args> + 'static,
    {
        self.register_periodic_worker_inner(builder, worker, args, Some(config))
            .await
    }

    async fn register_periodic_worker_inner<Args, W>(
        mut self,
        builder: periodic::Builder,
        worker: W,
        args: Args,
        config: Option<AppWorkerConfig>,
    ) -> RoadsterResult<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<S, Args> + 'static,
    {
        if let BuilderState::Enabled {
            processor,
            state,
            registered_periodic_workers,
            ..
        } = &mut self.state
        {
            let context = AppContext::from_ref(state);
            let class_name = W::class_name();
            debug!(worker = %class_name, "Registering periodic worker");
            let roadster_worker = RoadsterWorker::new(&context, worker, config);
            let builder = builder.args(args)?;
            let job_json = serde_json::to_string(&builder.into_periodic_job(class_name.clone())?)?;
            if !registered_periodic_workers.insert(job_json.clone()) {
                return Err(anyhow!(
                    "Periodic worker `{class_name}` was already registered; full job: {job_json}"
                )
                .into());
            }
            processor
                .register_periodic(builder, roadster_worker)
                .await?;
        }

        Ok(self)
    }

    pub async fn middleware<M>(mut self, middleware: M) -> RoadsterResult<Self>
    where
        M: ServerMiddleware + Send + Sync + 'static,
    {
        if let BuilderState::Enabled { processor, .. } = &mut self.state {
            processor.middleware(middleware).await;
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use crate::service::worker::sidekiq::processor_wrapper::MockProcessorWrapper;
    use bb8::Pool;
    use futures::StreamExt;
    use rstest::rstest;
    use sidekiq::{RedisConnectionManager, Worker};

    #[rstest]
    #[case(true, 1, vec![MockTestAppWorker::class_name()])]
    #[case(false, 0, Default::default())]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn register_app_worker(
        #[case] enabled: bool,
        #[case] expected_size: usize,
        #[case] expected_class_names: Vec<String>,
    ) {
        // Arrange
        let builder = setup(enabled, expected_size, 0).await;

        // Act
        let builder = builder
            .register_worker(MockTestAppWorker::default())
            .unwrap();

        // Assert
        validate_registered_workers(&builder, enabled, expected_size, expected_class_names);
        validate_registered_periodic_workers(&builder, enabled, 0, Default::default());
    }

    #[tokio::test]
    #[should_panic]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn register_app_worker_register_twice() {
        // Arrange
        let builder = setup(true, 2, 0).await;

        // Act
        builder
            .register_worker(MockTestAppWorker::default())
            .unwrap()
            .register_worker(MockTestAppWorker::default())
            .unwrap();
    }

    #[rstest]
    #[case(true, vec!["foo".to_string()])]
    #[case(true, vec!["foo".to_string(), "bar".to_string()])]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn register_periodic_app_worker(#[case] enabled: bool, #[case] job_names: Vec<String>) {
        // Arrange
        let builder = setup(true, 0, job_names.len()).await;

        // Act
        let builder = futures::stream::iter(job_names.clone())
            .fold(builder, |builder, name| async move {
                builder
                    .register_periodic_worker(
                        periodic::builder("* * * * * *").unwrap().name(name),
                        MockTestAppWorker::default(),
                        (),
                    )
                    .await
                    .unwrap()
            })
            .await;

        // Assert
        validate_registered_workers(&builder, enabled, 0, Default::default());
        validate_registered_periodic_workers(&builder, enabled, job_names.len(), job_names)
    }

    mockall::mock! {
        TestAppWorker{}

        #[async_trait]
        impl Worker<()> for TestAppWorker {
            async fn perform(&self, args: ()) -> sidekiq::Result<()>;
        }
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn setup(
        enabled: bool,
        register_count: usize,
        periodic_count: usize,
    ) -> SidekiqWorkerServiceBuilder<AppContext> {
        let mut config = AppConfig::test(None).unwrap();
        config.service.default_enable = enabled;
        config.service.sidekiq.custom.num_workers = 1;
        config.service.sidekiq.custom.queues = vec!["foo".to_string()];

        let redis_fetch = RedisConnectionManager::new("redis://invalid_host:1234").unwrap();
        let pool = Pool::builder().build_unchecked(redis_fetch);
        let context = AppContext::test(Some(config), None, Some(pool)).unwrap();

        let mut processor = MockProcessorWrapper::default();
        processor
            .expect_register::<(), MockTestAppWorker>()
            .times(register_count)
            .returning(|_| ());
        processor
            .expect_register_periodic::<(), MockTestAppWorker>()
            .times(periodic_count)
            .returning(|_, _| Ok(()));

        SidekiqWorkerServiceBuilder::new(context, Some(processor))
            .await
            .unwrap()
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn validate_registered_workers(
        builder: &SidekiqWorkerServiceBuilder<AppContext>,
        enabled: bool,
        size: usize,
        class_names: Vec<String>,
    ) {
        match &builder.state {
            BuilderState::Enabled {
                registered_workers, ..
            } => {
                assert!(enabled, "Builder should be disabled!");
                assert_eq!(registered_workers.len(), size);
                class_names
                    .iter()
                    .for_each(|class_name| assert!(registered_workers.contains(class_name)));
            }
            BuilderState::Disabled => {
                assert!(!enabled, "Builder should not be disabled!");
            }
        }
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn validate_registered_periodic_workers(
        builder: &SidekiqWorkerServiceBuilder<AppContext>,
        enabled: bool,
        size: usize,
        job_names: Vec<String>,
    ) {
        match &builder.state {
            BuilderState::Enabled {
                registered_periodic_workers,
                ..
            } => {
                assert!(enabled, "Builder should be disabled!");
                assert_eq!(registered_periodic_workers.len(), size);
                job_names.iter().for_each(|job_string| {
                    assert!(registered_periodic_workers
                        .iter()
                        .any(|registered| registered.contains(job_string)));
                });
            }
            BuilderState::Disabled => {
                assert!(!enabled, "Builder should not be disabled!");
            }
        }
    }

    #[rstest]
    #[case(true, true)]
    #[case(false, false)]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn clean_up_periodic_jobs_already_registered(
        #[case] enabled: bool,
        #[case] expect_err: bool,
    ) {
        // Arrange
        let register_count = if enabled { 1 } else { 0 };
        let builder = setup(enabled, 0, register_count).await;
        let builder = if enabled {
            builder
                .register_periodic_worker(
                    periodic::builder("* * * * * *").unwrap().name("foo"),
                    MockTestAppWorker::default(),
                    (),
                )
                .await
                .unwrap()
        } else {
            builder
        };

        // Act
        let result = builder.clean_up_periodic_jobs().await;

        // Assert
        assert_eq!(result.is_err(), expect_err);
    }
}
