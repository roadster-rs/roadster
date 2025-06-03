use crate::app::context::AppContext;
use crate::config::AppConfig;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

#[async_trait]
pub trait Worker<S, Args>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Serialize + for<'de> Deserialize<'de>,
{
    type Error;
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
