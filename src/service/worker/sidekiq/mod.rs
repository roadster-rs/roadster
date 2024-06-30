use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::worker::sidekiq::app_worker::AppWorker;
use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
use axum::extract::FromRef;
use serde::Serialize;
use sidekiq::{periodic, ServerMiddleware};

pub mod app_worker;
pub mod builder;
pub mod roadster_worker;
pub mod service;

/// A wrapper around [sidekiq::Processor] to help with mocking; we can't simply mock
/// sidekiq::Processor because [periodic::Builder] takes a [sidekiq::Processor] in order
/// to register a periodic job, so it won't be albe to take a MockProcessor created by `mockall`.
#[derive(Clone)]
struct Processor {
    inner: sidekiq::Processor,
}

impl Processor {
    #[cfg_attr(test, allow(dead_code))]
    fn new(inner: sidekiq::Processor) -> Self {
        Self { inner }
    }

    #[cfg_attr(test, allow(dead_code))]
    fn register<S, Args, W>(&mut self, worker: RoadsterWorker<S, Args, W>)
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<S, Args> + 'static,
    {
        self.inner.register(worker);
    }

    #[cfg_attr(test, allow(dead_code))]
    async fn register_periodic<S, Args, W>(
        &mut self,
        builder: periodic::Builder,
        worker: RoadsterWorker<S, Args, W>,
    ) -> RoadsterResult<()>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
        W: AppWorker<S, Args> + 'static,
    {
        builder.register(&mut self.inner, worker).await?;
        Ok(())
    }

    #[cfg_attr(test, allow(dead_code))]
    async fn middleware<M>(&mut self, middleware: M)
    where
        M: ServerMiddleware + Send + Sync + 'static,
    {
        self.inner.using(middleware).await;
    }

    #[cfg_attr(test, allow(dead_code))]
    fn into_sidekiq_processor(self) -> sidekiq::Processor {
        self.inner
    }
}

#[cfg(test)]
mockall::mock! {
    Processor {
        fn new(inner: sidekiq::Processor) -> Self;

        fn register<S, Args, W>(&mut self, worker: RoadsterWorker<S, Args, W>)
        where
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
            W: AppWorker<S, Args> + 'static;

        async fn register_periodic<S, Args, W>(
            &mut self,
            builder: periodic::Builder,
            worker: RoadsterWorker<S, Args, W>,
        ) -> RoadsterResult<()>
        where
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Sync + Send + Serialize + for<'de> serde::Deserialize<'de> + 'static,
            W: AppWorker<S, Args> + 'static;

        async fn middleware<M>(&mut self, middleware: M)
        where
            M: ServerMiddleware + Send + Sync + 'static;

        fn into_sidekiq_processor(self) -> sidekiq::Processor;
    }

    impl Clone for Processor {
        fn clone(&self) -> Self;
    }
}
