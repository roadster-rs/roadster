use crate::app::App;
use crate::worker::app_worker::AppWorker;
use crate::worker::RoadsterWorker;
use serde::Serialize;
use sidekiq::{periodic, Processor};
use std::sync::Arc;
use tracing::debug;

/// Custom wrapper around [Processor] to help with registering [workers][AppWorker] that are
/// wrapped by [RoadsterWorker].
pub struct WorkerRegistry<A>
where
    A: App + ?Sized,
{
    pub(crate) processor: Processor,
    pub(crate) state: Arc<A::State>,
}

impl<A> WorkerRegistry<A>
where
    A: App + 'static,
{
    /// Register a [worker][AppWorker] to handle Sidekiq.rs jobs.
    ///
    /// The worker will be wrapped by a [RoadsterWorker], which provides some common behavior, such
    /// as enforcing a timeout/max duration of worker jobs.
    pub fn register_app_worker<Args, W>(&mut self, worker: W)
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        debug!("Registering worker: `{}`", W::class_name());
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
        debug!("Registering periodic worker: `{}`", W::class_name());
        let roadster_worker = RoadsterWorker::new(worker, self.state.clone());
        builder
            .args(args)?
            .register(&mut self.processor, roadster_worker)
            .await?;
        Ok(())
    }
}
