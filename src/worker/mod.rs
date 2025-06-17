use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use axum_core::extract::FromRef;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use validator::Validate;

pub mod backend;
pub(crate) mod enqueue;
pub(crate) mod job;
pub(crate) mod worker;

pub use enqueue::Enqueuer;
pub use worker::{EnqueueConfig, Worker, WorkerConfig};

type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S, &'a str) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

struct Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    workers: HashMap<String, WorkerFn<S>>,
}

impl<S> Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: std::error::Error + Send + Sync,
    {
        // todo: can we get rid of the `Arc` (and the `clones` below)?
        let worker = Arc::new(worker);
        self.workers.insert(
            W::name(),
            // Todo: instrument to allow recording spans/metrics
            Box::new(move |state: &S, args: &str| {
                let worker = worker.clone();
                Box::pin(async move {
                    let args: Args = serde_json::from_str(args)?;
                    match worker.clone().handle(&state, &args).await {
                        Ok(_) => Ok(()),
                        // todo: timeouts, etc
                        Err(err) => Err(crate::error::worker::WorkerError::Handle(
                            W::name(),
                            Box::new(err),
                        )
                        .into()),
                    }
                })
            }),
        );
        Ok(())
    }
}
