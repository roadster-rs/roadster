use crate::app::App;
#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::service::worker::sidekiq::StaleCleanUpBehavior;
use crate::service::worker::sidekiq::app_worker::AppWorker;
use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
use crate::service::worker::sidekiq::service::SidekiqWorkerService;
#[mockall_double::double]
use crate::service::worker::sidekiq::Processor;
use crate::service::{AppService, AppServiceBuilder};
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use itertools::Itertools;
use num_traits::ToPrimitive;
use serde::Serialize;
use sidekiq::{periodic, ProcessorConfig};
use std::collections::HashSet;
use tracing::{debug, info, warn};

const PERIODIC_KEY: &str = "periodic";

pub struct SidekiqWorkerServiceBuilder<A>
where
    A: App + 'static,
{
    state: BuilderState<A>,
}

enum BuilderState<A: App + 'static> {
    Enabled {
        processor: Processor<A>,
        context: AppContext<A::State>,
        registered_workers: HashSet<String>,
        registered_periodic_workers: HashSet<String>,
    },
    Disabled,
}

#[async_trait]
impl<A> AppServiceBuilder<A, SidekiqWorkerService> for SidekiqWorkerServiceBuilder<A>
where
    A: App,
{
    fn enabled(&self, app_context: &AppContext<A::State>) -> bool {
        match self.state {
            BuilderState::Enabled { .. } => {
                <SidekiqWorkerService as AppService<A>>::enabled(app_context)
            }
            BuilderState::Disabled => false,
        }
    }

    async fn build(self, context: &AppContext<A::State>) -> anyhow::Result<SidekiqWorkerService> {
        let service = match self.state {
            BuilderState::Enabled {
                processor,
                registered_periodic_workers,
                ..
            } => {
                Self::remove_stale_periodic_jobs(context, &registered_periodic_workers).await?;
                SidekiqWorkerService {
                    processor: processor.into_sidekiq_processor(),
                }
            }
            BuilderState::Disabled => {
                bail!("This builder is not enabled; it's build method should not have been called.")
            }
        };

        Ok(service)
    }
}

impl<A> SidekiqWorkerServiceBuilder<A>
where
    A: App + 'static,
{
    pub async fn with_processor(
        context: &AppContext<A::State>,
        processor: sidekiq::Processor,
    ) -> anyhow::Result<Self> {
        Self::new(context.clone(), Some(Processor::new(processor))).await
    }

    pub async fn with_default_processor(
        context: &AppContext<A::State>,
        worker_queues: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        let processor = if !<SidekiqWorkerService as AppService<A>>::enabled(context) {
            debug!("Sidekiq service not enabled, not creating the Sidekiq processor");
            None
        } else if let Some(redis_fetch) = context.redis_fetch() {
            Self::auto_clean_periodic(context).await?;
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
                let num_workers = context
                    .config()
                    .service
                    .sidekiq
                    .custom
                    .num_workers
                    .to_usize()
                    .ok_or_else(|| {
                        anyhow!(
                            "Unable to convert num_workers `{}` to usize",
                            context.config().service.sidekiq.custom.num_workers
                        )
                    })?;
                let processor_config: ProcessorConfig = Default::default();
                let processor_config = processor_config.num_workers(num_workers);
                let processor = sidekiq::Processor::new(redis_fetch.clone(), queues.clone())
                    .with_config(processor_config);
                Processor::new(processor)
            };

            Some(processor)
        } else {
            debug!(
                "No 'redis fetch' connection pool configured, not creating the Sidekiq processor"
            );
            None
        };

        Self::new(context.clone(), processor).await
    }

    async fn new(
        context: AppContext<A::State>,
        processor: Option<Processor<A>>,
    ) -> anyhow::Result<Self> {
        let processor = if <SidekiqWorkerService as AppService<A>>::enabled(&context) {
            processor
        } else {
            None
        };

        let state = if let Some(processor) = processor {
            BuilderState::Enabled {
                processor,
                context,
                registered_workers: Default::default(),
                registered_periodic_workers: Default::default(),
            }
        } else {
            BuilderState::Disabled
        };

        Ok(Self { state })
    }

    async fn auto_clean_periodic(context: &AppContext<A::State>) -> anyhow::Result<()> {
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
            periodic::destroy_all(context.redis_enqueue().clone()).await?;
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
    pub async fn clean_up_periodic_jobs(self) -> anyhow::Result<Self> {
        if let BuilderState::Enabled {
            registered_periodic_workers,
            context,
            ..
        } = &self.state
        {
            if !registered_periodic_workers.is_empty() {
                bail!("Can only clean up previous periodic jobs if no periodic jobs have been registered yet.")
            }
            periodic::destroy_all(context.redis_enqueue().clone()).await?;
        }

        Ok(self)
    }

    /// Register a [worker][AppWorker] to handle Sidekiq.rs jobs.
    ///
    /// The worker will be wrapped by a [RoadsterWorker], which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs.
    pub fn register_app_worker<Args, W>(mut self, worker: W) -> anyhow::Result<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        if let BuilderState::Enabled {
            processor,
            registered_workers,
            context,
            ..
        } = &mut self.state
        {
            let class_name = W::class_name();
            debug!(worker = %class_name, "Registering worker");
            if !registered_workers.insert(class_name.clone()) {
                bail!("Worker `{class_name}` was already registered");
            }
            let roadster_worker = RoadsterWorker::new(worker, context);
            processor.register(roadster_worker);
        }

        Ok(self)
    }

    /// Register a periodic [worker][AppWorker] that will run with the provided args. The cadence
    /// of the periodic worker, the worker's queue name, and other attributes are specified using
    /// the [builder][periodic::Builder]. However, to help ensure type-safety the args are provided
    /// to this method instead of the [builder][periodic::Builder].
    ///
    /// The worker will be wrapped by a [RoadsterWorker], which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs.
    pub async fn register_periodic_app_worker<Args, W>(
        mut self,
        builder: periodic::Builder,
        worker: W,
        args: Args,
    ) -> anyhow::Result<Self>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        if let BuilderState::Enabled {
            processor,
            context,
            registered_periodic_workers,
            ..
        } = &mut self.state
        {
            let class_name = W::class_name();
            debug!(worker = %class_name, "Registering periodic worker");
            let roadster_worker = RoadsterWorker::new(worker, context);
            let builder = builder.args(args)?;
            let job_json = serde_json::to_string(&builder.into_periodic_job(class_name.clone())?)?;
            if !registered_periodic_workers.insert(job_json.clone()) {
                bail!(
                    "Periodic worker `{class_name}` was already registered; full job: {job_json}"
                );
            }
            processor
                .register_periodic(builder, roadster_worker)
                .await?;
        }

        Ok(self)
    }

    /// Compares the list of periodic jobs that were registered by the app during app startup with
    /// the list of periodic jobs in Redis, and removes any that exist in Redis but weren't
    /// registered during start up.
    ///
    /// The jobs are only removed if the [worker.sidekiq.periodic.stale-cleanup][crate::config::worker::Periodic]
    /// config is set to [auto-clean-stale][StaleCleanUpBehavior::AutoCleanStale].
    ///
    /// This is run after all the app's periodic jobs have been registered.
    pub(crate) async fn remove_stale_periodic_jobs(
        context: &AppContext<A::State>,
        registered_periodic_workers: &HashSet<String>,
    ) -> anyhow::Result<()> {
        let mut conn = context.redis_enqueue().get().await?;
        let stale_jobs = conn
            .zrange(PERIODIC_KEY.to_string(), 0, -1)
            .await?
            .into_iter()
            .filter(|job| !registered_periodic_workers.contains(job))
            .collect_vec();

        if stale_jobs.is_empty() {
            info!("No stale periodic jobs found");
            return Ok(());
        }

        if context
            .config()
            .service
            .sidekiq
            .custom
            .periodic
            .stale_cleanup
            == StaleCleanUpBehavior::AutoCleanStale
        {
            info!(
                "Removing {} stale periodic jobs:\n{}",
                stale_jobs.len(),
                stale_jobs.join("\n")
            );
            conn.zrem(PERIODIC_KEY.to_string(), &stale_jobs).await?;
        } else {
            warn!(
                "Found {} stale periodic jobs:\n{}",
                stale_jobs.len(),
                stale_jobs.join("\n")
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockTestApp;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use crate::service::worker::sidekiq::MockProcessor;
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
            .register_app_worker(MockTestAppWorker::default())
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
            .register_app_worker(MockTestAppWorker::default())
            .unwrap()
            .register_app_worker(MockTestAppWorker::default())
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
                    .register_periodic_app_worker(
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

        #[async_trait]
        impl AppWorker<MockTestApp, ()> for TestAppWorker
        {
            fn build(context: &MockAppContext<()>) -> Self;
        }
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn setup(
        enabled: bool,
        register_count: usize,
        periodic_count: usize,
    ) -> SidekiqWorkerServiceBuilder<MockTestApp> {
        let mut config = AppConfig::empty(None).unwrap();
        config.service.default_enable = enabled;
        config.service.sidekiq.custom.num_workers = 1;
        config.service.sidekiq.custom.queues = vec!["foo".to_string()];

        let mut context = MockAppContext::default();
        context.expect_config().return_const(config);
        let redis_fetch = RedisConnectionManager::new("redis://invalid_host:1234").unwrap();
        let pool = Pool::builder().build_unchecked(redis_fetch);
        context.expect_redis_fetch().return_const(Some(pool));

        let mut processor = MockProcessor::<MockTestApp>::default();
        processor
            .expect_register::<(), MockTestAppWorker>()
            .times(register_count)
            .returning(|_| ());
        processor
            .expect_register_periodic::<(), MockTestAppWorker>()
            .times(periodic_count)
            .returning(|_, _| Ok(()));

        SidekiqWorkerServiceBuilder::<MockTestApp>::new(context, Some(processor))
            .await
            .unwrap()
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn validate_registered_workers(
        builder: &SidekiqWorkerServiceBuilder<MockTestApp>,
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
        builder: &SidekiqWorkerServiceBuilder<MockTestApp>,
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
}
