use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::worker::{EnqueueConfig, Worker};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/*
Todo: how much do we want to abstract the actions we need to take on a queue? Yes, we need
 to enqueue, but we'll also need to dequeue, possibly re-enqueue, create queues (for pgmq), etc.
 Do we want that to all be abstracted? Also, for backends that have their own processors
 (e.g. sidekiq), do we need to wrap them in an abstraction, and/or, should we have our own
 common/custom processor that can work for all backends using their abstractions?
*/
#[async_trait]
trait Enqueuer<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    async fn enqueue<W, Args, E>(state: &S, args: &Args) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>;

    async fn enqueue_delayed<W, Args, E>(
        state: &S,
        args: &Args,
        delay: Duration,
    ) -> RoadsterResult<()>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>;
}
