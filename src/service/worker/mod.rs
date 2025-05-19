use crate::error::RoadsterResult;
use async_trait::async_trait;

#[cfg(feature = "pg-queue")]
pub mod pg;
#[cfg(feature = "sidekiq")]
pub mod sidekiq;

#[async_trait]
pub trait Worker<Args> {
    // todo: Make this roadster specific and pass the app-state as a method param? That would
    //  certainly make it a bit easier to use, which would be nice.
    // todo: Make general enough to work as a shared/wrapper trait of sidekiq's worker trait?
    async fn handle(args: Args) -> RoadsterResult<()>;
}
