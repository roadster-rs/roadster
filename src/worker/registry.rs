use crate::app::App;
use crate::worker::app_worker::AppWorker;
use crate::worker::RoadsterWorker;
use serde::Serialize;
use sidekiq::Processor;
use std::sync::Arc;

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
    pub fn register_app_worker<Args, W>(&mut self, worker: W)
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        let roadster_worker = RoadsterWorker::new(worker, self.state.clone());
        self.processor.register(roadster_worker);
    }
}
