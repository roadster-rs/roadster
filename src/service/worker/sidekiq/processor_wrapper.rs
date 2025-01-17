use crate::error::RoadsterResult;
use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
use serde::Serialize;
use sidekiq::{periodic, ServerMiddleware, Worker};

/// A wrapper around [sidekiq::Processor] to help with mocking; we can't simply mock
/// sidekiq::Processor because [periodic::Builder] takes a [sidekiq::Processor] in order
/// to register a periodic job, so it won't be albe to take a MockProcessor created by `mockall`.
#[derive(Clone)]
pub(crate) struct ProcessorWrapper {
    inner: sidekiq::Processor,
}

impl ProcessorWrapper {
    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn new(inner: sidekiq::Processor) -> Self {
        Self { inner }
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn register<Args, W>(&mut self, worker: RoadsterWorker<Args, W>)
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: Worker<Args> + 'static,
    {
        self.inner.register(worker);
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) async fn register_periodic<Args, W>(
        &mut self,
        builder: periodic::Builder,
        worker: RoadsterWorker<Args, W>,
    ) -> RoadsterResult<()>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: Worker<Args> + 'static,
    {
        builder.register(&mut self.inner, worker).await?;
        Ok(())
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) async fn middleware<M>(&mut self, middleware: M)
    where
        M: ServerMiddleware + Send + Sync + 'static,
    {
        self.inner.using(middleware).await;
    }

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn into_sidekiq_processor(self) -> sidekiq::Processor {
        self.inner
    }
}

#[cfg(test)]
mockall::mock! {
    pub(crate) ProcessorWrapper {
        pub(crate) fn new(inner: sidekiq::Processor) -> Self;

        pub(crate) fn register<Args, W>(&mut self, worker: RoadsterWorker<Args, W>)
        where
            Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
            W: Worker<Args> + 'static;

        pub(crate) async fn register_periodic<Args, W>(
            &mut self,
            builder: periodic::Builder,
            worker: RoadsterWorker<Args, W>,
        ) -> RoadsterResult<()>
        where
            Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
            W: Worker<Args> + 'static;

        pub(crate) async fn middleware<M>(&mut self, middleware: M)
        where
            M: ServerMiddleware + Send + Sync + 'static;

        pub(crate) fn into_sidekiq_processor(self) -> sidekiq::Processor;
    }

    impl Clone for ProcessorWrapper {
        fn clone(&self) -> Self;
    }
}
