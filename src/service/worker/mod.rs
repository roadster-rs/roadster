use crate::app::context::AppContext;
use crate::config::AppConfig;
use crate::error::RoadsterResult;
use crate::util::serde::deserialize_from_str;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use pgmq::PGMQueue;
use serde::{Deserialize, Serialize};
use std::any::{Any, type_name, type_name_of_val};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

#[async_trait]
pub trait Worker<S, Args>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Serialize + for<'de> Deserialize<'de>,
{
    type Error: std::error::Error;

    async fn handle(&self, state: &S, args: Args) -> Result<(), Self::Error>;

    async fn enqueue(state: &S, args: Args) -> Result<(), Self::Error>
    where
        Self: Sized;

    async fn enqueue_delayed(state: &S, args: Args, delay: Duration) -> Result<(), Self::Error>
    where
        Self: Sized;
}

struct Foo;

#[async_trait]
impl Worker<AppContext, ()> for Foo {
    type Error = crate::error::Error;

    async fn handle(&self, state: &AppContext, args: ()) -> Result<(), Self::Error> {
        todo!()
    }

    async fn enqueue(state: &AppContext, args: ()) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        todo!()
    }

    async fn enqueue_delayed(
        state: &AppContext,
        args: (),
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        todo!()
    }
}

// Todo: How to store workers when they all have different args and possibly different error types?
fn foo() -> Vec<Box<dyn Worker<AppContext, (), Error = crate::error::Error>>> {
    let foo = Foo;
    let foo: Box<dyn Worker<AppContext, (), Error = crate::error::Error>> = Box::new(foo);
    let foo = vec![foo];
    foo
}

struct Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    workers: HashMap<
        String,
        Box<
            dyn Send
                + Sync
                + for<'a> Fn(
                    &'a S,
                    String,
                )
                    -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
        >,
    >,
}

impl<S> Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn register<W, Args, E>(mut self, worker: W)
    where
        // todo: can we get rid of the `'static`?
        W: 'static + Worker<S, Args, Error = E>,
        AppContext: FromRef<S>,
        Args: Serialize + for<'de> Deserialize<'de>,
    {
        // todo: can we get rid of the `Arc` (and the `clones` below)?
        let worker = Arc::new(worker);
        self.workers.insert(
            type_name_of_val(&worker).to_string(),
            Box::new(move |state: &S, args: String| {
                let worker = worker.clone();
                Box::pin(async move {
                    let args: Args = serde_json::from_str(&args)?;
                    match worker.clone().handle(&state, args).await {
                        Ok(_) => Ok(()),
                        // Todo: better error handling
                        // todo: timeouts, etc
                        Err(err) => Err(anyhow!("foo").into()),
                    }
                })
            }),
        );
    }
}
