//! Background task queue service backed by Postgres using [pgmq](https://docs.rs/pgmq).

/*
- job/task/message
- runner/worker/handler/processor
 */
use crate::error::RoadsterResult;
use async_trait::async_trait;

#[async_trait]
pub trait Worker<Args> {
    // todo: Make this roadster specific and pass the app-state as a method param? That would
    //  certainly make it a bit easier to use, which would be nice.
    // todo: Make general enough to work as a shared/wrapper trait of sidekiq's worker trait?
    async fn handle(args: Args) -> RoadsterResult<()>;
}
