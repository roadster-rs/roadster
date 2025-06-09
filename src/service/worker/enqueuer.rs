use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::worker::Worker;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
