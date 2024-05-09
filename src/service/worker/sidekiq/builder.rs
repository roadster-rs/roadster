use crate::app::App;
use crate::app_context::AppContext;
use crate::config::service::worker::sidekiq::StaleCleanUpBehavior;
use crate::service::worker::sidekiq::app_worker::AppWorker;
use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
use crate::service::worker::sidekiq::service::SidekiqWorkerService;
use crate::service::{AppService, AppServiceBuilder};
use anyhow::{anyhow, bail};
use async_trait::async_trait;
use itertools::Itertools;
use num_traits::ToPrimitive;
use serde::Serialize;
use sidekiq::{periodic, Processor, ProcessorConfig};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, warn};

const PERIODIC_KEY: &str = "periodic";

pub struct SidekiqWorkerServiceBuilder<A>
where
    A: App,
{
    state: BuilderState<A>,
}

enum BuilderState<A: App> {
    Enabled {
        processor: Processor,
        context: Arc<AppContext>,
        state: Arc<A::State>,
        registered_workers: HashSet<String>,
        registered_periodic_workers: HashSet<String>,
    },
    Disabled,
}

#[async_trait]
impl<A> AppServiceBuilder<A, SidekiqWorkerService> for SidekiqWorkerServiceBuilder<A>
where
    A: App + 'static,
{
    fn enabled(&self, app_context: &AppContext, app_state: &A::State) -> bool {
        match self.state {
            BuilderState::Enabled { .. } => {
                <SidekiqWorkerService as AppService<A>>::enabled(app_context, app_state)
            }
            BuilderState::Disabled => false,
        }
    }

    async fn build(
        self,
        context: &AppContext,
        _state: &A::State,
    ) -> anyhow::Result<SidekiqWorkerService> {
        let service = match self.state {
            BuilderState::Enabled {
                processor,
                registered_periodic_workers,
                ..
            } => {
                Self::remove_stale_periodic_jobs(context, &registered_periodic_workers).await?;
                SidekiqWorkerService { processor }
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
        context: Arc<AppContext>,
        state: Arc<A::State>,
        processor: Processor,
    ) -> anyhow::Result<Self> {
        Self::new(context, state, Some(processor)).await
    }

    pub async fn with_default_processor(
        context: Arc<AppContext>,
        state: Arc<A::State>,
        worker_queues: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        let processor = if !<SidekiqWorkerService as AppService<A>>::enabled(&context, &state) {
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
                Processor::new(redis_fetch.clone(), queues.clone()).with_config(processor_config)
            };

            Some(processor)
        } else {
            debug!(
                "No 'redis fetch' connection pool configured, not creating the Sidekiq processor"
            );
            None
        };

        Self::new(context, state, processor).await
    }

    async fn new(
        context: Arc<AppContext>,
        state: Arc<A::State>,
        processor: Option<Processor>,
    ) -> anyhow::Result<Self> {
        let processor = if <SidekiqWorkerService as AppService<A>>::enabled(&context, &state) {
            processor
        } else {
            None
        };

        let state = if let Some(processor) = processor {
            BuilderState::Enabled {
                processor,
                context,
                state,
                registered_workers: Default::default(),
                registered_periodic_workers: Default::default(),
            }
        } else {
            BuilderState::Disabled
        };

        Ok(Self { state })
    }

    async fn auto_clean_periodic(context: &AppContext) -> anyhow::Result<()> {
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
            state,
            registered_workers,
            ..
        } = &mut self.state
        {
            let class_name = W::class_name();
            debug!(worker = %class_name, "Registering worker");
            if !registered_workers.insert(class_name.clone()) {
                bail!("Worker `{class_name}` was already registered");
            }
            let roadster_worker = RoadsterWorker::new(worker, state.clone());
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
            state,
            registered_periodic_workers,
            ..
        } = &mut self.state
        {
            let class_name = W::class_name();
            debug!(worker = %class_name, "Registering periodic worker");
            let roadster_worker = RoadsterWorker::new(worker, state.clone());
            let builder = builder.args(args)?;
            let job_json = serde_json::to_string(&builder.into_periodic_job(class_name.clone())?)?;
            if !registered_periodic_workers.insert(job_json.clone()) {
                bail!(
                    "Periodic worker `{class_name}` was already registered; full job: {job_json}"
                );
            }
            builder.register(processor, roadster_worker).await?;
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
        context: &AppContext,
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
