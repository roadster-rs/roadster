use crate::app::App;
use crate::app_context::AppContext;
use crate::config::worker::StaleCleanUpBehavior;
use crate::worker::app_worker::AppWorker;
use crate::worker::RoadsterWorker;
use itertools::Itertools;
use serde::Serialize;
use sidekiq::{periodic, Processor};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, info, warn};

const PERIODIC_KEY: &str = "periodic";

/// Custom wrapper around [Processor] to help with registering [workers][AppWorker] that are
/// wrapped by [RoadsterWorker].
pub struct WorkerRegistry<A>
where
    A: App + ?Sized,
{
    pub(crate) processor: Processor,
    pub(crate) state: Arc<A::State>,
    pub(crate) registered_workers: HashSet<String>,
    pub(crate) registered_periodic_workers: HashSet<String>,
}

impl<A> WorkerRegistry<A>
where
    A: App + 'static,
{
    pub(crate) fn new(processor: Processor, state: Arc<A::State>) -> Self {
        Self {
            processor,
            state,
            registered_workers: Default::default(),
            registered_periodic_workers: Default::default(),
        }
    }

    /// Register a [worker][AppWorker] to handle Sidekiq.rs jobs.
    ///
    /// The worker will be wrapped by a [RoadsterWorker], which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs.
    pub fn register_app_worker<Args, W>(&mut self, worker: W)
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        let class_name = W::class_name();
        debug!(worker = class_name, "Registering worker");
        self.registered_workers.insert(class_name.clone());
        let roadster_worker = RoadsterWorker::new(worker, self.state.clone());
        self.processor.register(roadster_worker);
    }

    /// Register a periodic [worker][AppWorker] that will run with the provided args. The cadence
    /// of the periodic worker, the worker's queue name, and other attributes are specified using
    /// the [builder][periodic::Builder]. However, to help ensure type-safety the args are provided
    /// to this method instead of the [builder][periodic::Builder].
    ///
    /// The worker will be wrapped by a [RoadsterWorker], which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs.
    pub async fn register_periodic_app_worker<Args, W>(
        &mut self,
        builder: periodic::Builder,
        worker: W,
        args: Args,
    ) -> anyhow::Result<()>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        let class_name = W::class_name();
        debug!(worker = class_name, "Registering periodic worker");
        let roadster_worker = RoadsterWorker::new(worker, self.state.clone());
        let builder = builder.args(args)?;
        let job_json = serde_json::to_string(&builder.into_periodic_job(class_name)?)?;
        self.registered_periodic_workers.insert(job_json);
        builder
            .register(&mut self.processor, roadster_worker)
            .await?;
        Ok(())
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
        &self,
        context: &AppContext,
    ) -> anyhow::Result<()> {
        let mut conn = context.redis.get().await?;
        let stale_jobs = conn
            .zrange(PERIODIC_KEY.to_string(), 0, -1)
            .await?
            .into_iter()
            .filter(|job| !self.registered_periodic_workers.contains(job))
            .collect_vec();

        if stale_jobs.is_empty() {
            info!("No stale periodic jobs found");
            return Ok(());
        }

        if context.config.worker.sidekiq.periodic.stale_cleanup
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
