use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::worker::Worker;
use crate::worker::enqueue::queue_from_config;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PgProcessorError {
    /// The provided [`Worker`] was already registered. Contains the [`Worker::name`]
    /// of the provided worker.
    #[error("The provided `Worker` was already registered: `{0}`")]
    AlreadyRegistered(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[non_exhaustive]
pub struct Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    workers: HashMap<String, WorkerWrapper<S>>,
}

impl<S> Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    pub fn new(state: &S) -> Self {
        Self {
            state: state.clone(),
            workers: Default::default(),
        }
    }

    pub async fn register<W, Args, E>(&mut self, worker: W) -> RoadsterResult<&mut Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        // Ensure the worker's queue's tables exist
        let queue = queue_from_config::<W, _, _, _>(&self.state)?;
        let context = AppContext::from_ref(&self.state);
        context.pgmq().create(&queue).await?;

        let name = W::name();
        info!(name, "Registering PG worker");

        if self
            .workers
            .insert(name.clone(), WorkerWrapper::new(worker))
            .is_some()
        {
            return Err(PgProcessorError::AlreadyRegistered(name).into());
        }
        Ok(self)
    }
}

type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S, &'a str) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

struct WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: WorkerFn<S>,
}

impl<S> WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn new<W, Args, E>(worker: W) -> Self
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        // Todo: without this `'static`, we're getting an internal compiler error
        E: 'static + std::error::Error + Send + Sync,
    {
        let worker = Arc::new(worker);

        Self {
            inner: Box::new(move |state: &S, args: &str| {
                let worker = worker.clone();
                Box::pin(async move {
                    let args: Args = serde_json::from_str(args)
                        .map_err(crate::error::worker::DequeueError::Serde)?;

                    match worker.clone().handle(state, &args).await {
                        Ok(_) => Ok(()),
                        // todo: timeouts, etc
                        Err(err) => Err(crate::error::Error::from(
                            crate::error::worker::WorkerError::Handle(W::name(), Box::new(err)),
                        )),
                    }
                })
            }),
        }
    }

    async fn handle(&self, state: &S, args: &str) -> RoadsterResult<()> {
        (self.inner)(state, args).await?;
        Ok(())
    }
}
