use crate::app::App;
use crate::service::worker::sidekiq::app_worker::AppWorker;
use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
use serde::Serialize;
use sidekiq::periodic;
use std::marker::PhantomData;

pub mod app_worker;
pub mod builder;
pub mod roadster_worker;
pub mod service;

/// A wrapper around [sidekiq::Processor] to help with mocking; we can't simply mock
/// sidekiq::Processor because [periodic::Builder] takes a [sidekiq::Processor] in order
/// to register a periodic job, so it won't be albe to take a MockProcessor created by `mockall`.
#[derive(Clone)]
struct Processor<A>
where
    A: App + 'static,
{
    inner: sidekiq::Processor,
    _app: PhantomData<A>,
}

impl<A> Processor<A>
where
    A: App + 'static,
{
    #[cfg_attr(test, allow(dead_code))]
    fn new(inner: sidekiq::Processor) -> Self {
        Self {
            inner,
            _app: PhantomData,
        }
    }

    #[cfg_attr(test, allow(dead_code))]
    fn register<Args, W>(&mut self, worker: RoadsterWorker<A, Args, W>)
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        self.inner.register(worker);
    }

    #[cfg_attr(test, allow(dead_code))]
    async fn register_periodic<Args, W>(
        &mut self,
        builder: periodic::Builder,
        worker: RoadsterWorker<A, Args, W>,
    ) -> anyhow::Result<()>
    where
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<A, Args> + 'static,
    {
        builder.register(&mut self.inner, worker).await?;
        Ok(())
    }

    #[cfg_attr(test, allow(dead_code))]
    fn into_sidekiq_processor(self) -> sidekiq::Processor {
        self.inner
    }
}

#[cfg(test)]
mockall::mock! {
    Processor<A: App + 'static> {
        fn new(inner: sidekiq::Processor) -> Self;

        fn register<Args, W>(&mut self, worker: RoadsterWorker<A, Args, W>)
        where
            Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
            W: AppWorker<A, Args> + 'static;

        async fn register_periodic<Args, W>(
            &mut self,
            builder: periodic::Builder,
            worker: RoadsterWorker<A, Args, W>,
        ) -> anyhow::Result<()>
        where
            Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
            W: AppWorker<A, Args> + 'static;

        fn into_sidekiq_processor(self) -> sidekiq::Processor;
    }

    impl<A: App + 'static> Clone for Processor<A> {
        fn clone(&self) -> Self;
    }
}
