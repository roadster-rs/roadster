use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

//
// #[derive(Debug, Default, TypedBuilder)]
// pub struct WorkerConfig {
//     pub max_retries: Option<u32>,
// }
//
// // todo: TBD how tightly to couple this worker trait to roadster and the app state
// #[async_trait]
// pub trait Worker<S, Args>
// where
//     Self: Sized,
//     for<'de> Args: Send + Sync + Serialize + Deserialize<'de> + 'static,
//     S: Clone + Send + Sync + 'static,
//     AppContext: FromRef<S>,
// {
//     type Error: std::error::Error;
//
//     fn config(&self, state: &S) -> WorkerConfig {
//         // Todo: fallback to config from state?
//         WorkerConfig::default()
//     }
//
//     fn handle(&self, state: &S, args: Args) -> Result<(), Self::Error>;
// }
//
// #[async_trait]
// pub trait Processor {
//     fn enqueue();
//     fn enqueue_delayed();
//     fn enqueue_at();
// }
//
trait SidekiqWorker {
    fn perform(&self);
}

trait PgWorker {
    fn handle(&self);
}

trait FaktoryWorker {
    fn run(&self);
}

trait BaseWorker {
    fn handle(&self) {}
}

impl<T> BaseWorker for T {}

trait RoadsterWorker<Args>: BaseWorker {
    fn handle(&self);
}

// impl<T, Args> ::sidekiq::Worker<Args> for T where T: RoadsterWorker {
//     async fn perform(&self, args: Args) -> ::sidekiq::Result<()> {
//         todo!()
//     }
// }

impl<T, Args> RoadsterWorker<Args> for T
where
    T: ::sidekiq::Worker<Args> + BaseWorker,
{
    fn handle(&self) {
        todo!()
    }
}
// impl<T, Args> RoadsterWorker<Args> for T where T: FaktoryWorker {}

fn register_roadster_worker<Args>(worker: impl RoadsterWorker<Args>) {}

fn register_sidekiq_worker<Args>(worker: impl ::sidekiq::Worker<Args>) {}

struct FooArgs;
struct FooWorker;
// impl RoadsterWorker<FooArgs> for FooWorker {}
#[async_trait::async_trait]
impl ::sidekiq::Worker<FooArgs> for FooWorker {
    async fn perform(&self, args: FooArgs) -> ::sidekiq::Result<()> {
        todo!()
    }
}

fn register_workers() {
    register_roadster_worker(FooWorker);
    register_sidekiq_worker(FooWorker);
}
