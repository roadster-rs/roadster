use crate::app::context::AppContext;
use crate::util::types;
#[cfg(feature = "worker-pg")]
pub use crate::worker::backend::pg::enqueue::PgEnqueuer;
#[cfg(feature = "worker-pg")]
pub use crate::worker::backend::pg::processor::PgProcessor;
#[cfg(feature = "worker-sidekiq")]
pub use crate::worker::backend::sidekiq::enqueue::SidekiqEnqueuer;
#[cfg(feature = "worker-sidekiq")]
pub use crate::worker::backend::sidekiq::processor::SidekiqProcessor;
use crate::worker::config::{EnqueueConfig, WorkerConfig};
use crate::worker::enqueue::Enqueuer;
use crate::worker::job::JobMetadata;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use cron::Schedule;
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
use tracing::{Instrument, error, error_span, instrument};

pub mod backend;
pub mod config;
pub mod enqueue;
pub(crate) mod job;

#[async_trait]
pub trait Worker<S, Args>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    type Error: std::error::Error + Send + Sync;
    type Enqueuer: Enqueuer + Send + Sync;

    /// The name of the worker. This will be encoded in the job data when it's enqueued the backing
    /// database (Redis/Postgres), and used to identify which type should handle a job when it's
    /// fetched from the queue. Therefore, it should be unique across the app, and care should be
    /// taken when refactoring.
    ///
    /// By default, [`Self::name`] returns the name of the type that implements the [`Worker`]
    /// trait. See [`types::simple_type_name`].
    ///
    /// This is not included in the [`EnqueueConfig`] because [`EnqueueConfig`] is included in
    /// the [`crate::config::AppConfig`] to allow defining defaults for the config values, but
    /// the name needs to be specified separately for each [`Worker`].
    fn name() -> String
    where
        Self: Sized,
    {
        types::simple_type_name::<Self>()
    }

    /// Get worker-specific configuration options to use when enqueuing a job. Any value not
    /// provided in the returned [`WorkerConfig`] will fall back to the value from the
    /// [`crate::config::AppConfig`].
    ///
    /// The [`Worker::enqueue_config`] method will be called when enqueuing a job for the worker.
    fn enqueue_config(_state: &S) -> EnqueueConfig {
        EnqueueConfig::default()
    }

    /// Get worker-specific configuration options to use when handling a job. Any value not provided
    /// in the returned [`WorkerConfig`] will fall back to the value from the
    /// [`crate::config::AppConfig`].
    ///
    /// The [`Worker::worker_config`] method will be called once for each worker when it is
    /// registered, and the config will be stored to be used when the worker handles a job.
    fn worker_config(&self, _state: &S) -> WorkerConfig {
        WorkerConfig::default()
    }

    #[instrument(skip_all)]
    async fn enqueue<ArgsRef>(
        state: &S,
        args: ArgsRef,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        Self::Enqueuer::enqueue::<Self, _, _, _, Self::Error>(state, args).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_delayed<ArgsRef>(
        state: &S,
        args: ArgsRef,
        delay: Duration,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        Self::Enqueuer::enqueue_delayed::<Self, _, _, _, Self::Error>(state, args, delay).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch<ArgsRef>(
        state: &S,
        args: &[ArgsRef],
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        Self::Enqueuer::enqueue_batch::<Self, _, _, _, Self::Error>(state, args).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch_delayed<ArgsRef>(
        state: &S,
        args: &[ArgsRef],
        delay: Duration,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
        ArgsRef: Send + Sync + Borrow<Args> + Serialize,
    {
        Self::Enqueuer::enqueue_batch_delayed::<Self, _, _, _, Self::Error>(state, args, delay)
            .await?;
        Ok(())
    }

    async fn handle(&self, state: &S, args: Args) -> Result<(), Self::Error>;

    /// This is a "private" API that's only intended for usage in Roadster's internal benchmarking suite.
    /// This method does not follow any semver guarantees.
    #[cfg(feature = "bench")]
    #[doc(hidden)]
    async fn on_complete(&self) {}
}

#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
            serde_json::Value,
        ) -> std::pin::Pin<
            Box<dyn 'a + Send + Future<Output = crate::error::RoadsterResult<()>>>,
        >,
>;

#[cfg(all(
    feature = "bench",
    any(feature = "worker-pg", feature = "worker-sidekiq")
))]
type OnCompleteFn =
    Box<dyn Send + Sync + Fn() -> std::pin::Pin<Box<dyn Send + Future<Output = ()>>>>;

#[derive(Clone)]
#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
pub(crate) struct WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: std::sync::Arc<WorkerWrapperInner<S>>,
}

#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
pub(crate) struct WorkerWrapperInner<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    name: String,
    type_id: std::any::TypeId,
    #[allow(dead_code)]
    enqueue_config: EnqueueConfig,
    worker_config: WorkerConfig,
    worker_fn: WorkerFn<S>,
    #[cfg(feature = "bench")]
    #[allow(dead_code)]
    on_complete_fn: OnCompleteFn,
}

#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
impl<S> WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn new<W, Args, E>(
        state: &S,
        worker: W,
        enqueue_config: EnqueueConfig,
    ) -> crate::error::RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        use std::any::Any;

        let worker = std::sync::Arc::new(worker);

        #[cfg(feature = "bench")]
        let worker2 = worker.clone();

        Ok(Self {
            inner: std::sync::Arc::new(WorkerWrapperInner {
                name: W::name(),
                type_id: worker.type_id(),
                enqueue_config,
                worker_config: worker.worker_config(state),
                worker_fn: Box::new(move |state: &S, args: serde_json::Value| {
                    let worker = worker.clone();
                    Box::pin(async move {
                        let args: Args = serde_json::from_value(args)
                            .map_err(crate::error::worker::DequeueError::Serde)?;

                        match worker.clone().handle(state, args).await {
                            Ok(_) => Ok(()),
                            Err(err) => Err(crate::error::Error::from(
                                crate::error::worker::WorkerError::Handle(W::name(), Box::new(err)),
                            )),
                        }
                    })
                }),
                #[cfg(feature = "bench")]
                on_complete_fn: Box::new(move || {
                    let worker = worker2.clone();
                    Box::pin(async move {
                        worker.clone().on_complete().await;
                    })
                }),
            }),
        })
    }

    async fn handle(
        &self,
        state: &S,
        job_metadata: &JobMetadata,
        args: serde_json::Value,
    ) -> crate::error::RoadsterResult<()> {
        let span_name = format!("WORKER {}::handle", self.inner.name);
        let context = AppContext::from_ref(state);
        let queue_name = self.inner.enqueue_config.queue.as_ref().or(context
            .config()
            .service
            .worker
            .enqueue_config
            .queue
            .as_ref());
        let span = error_span!(
            "WORKER",
            otel.name = span_name,
            otel.kind = "CONSUMER",
            job.id = %job_metadata.id,
            worker.name = self.inner.name,
            worker.queue.name = queue_name
        );

        async {
            let inner = AssertUnwindSafe((self.inner.worker_fn)(state, args)).catch_unwind();

            let context = AppContext::from_ref(state);
            let timeout = self
                .inner
                .worker_config
                .timeout
                .or(context.config().service.worker.worker_config.timeout)
                .unwrap_or_default();

            let max_duration = if timeout {
                self.inner.worker_config.max_duration.or(context
                    .config()
                    .service
                    .worker
                    .worker_config
                    .max_duration)
            } else {
                None
            };

            let result = if let Some(max_duration) = max_duration {
                tokio::time::timeout(max_duration, inner)
                    .await
                    .map_err(|_| {
                        error!(
                            worker.name = self.inner.name,
                            worker.max_duration = max_duration.as_secs(),
                            "Worker timed out"
                        );
                        crate::error::worker::WorkerError::Timeout(
                            self.inner.name.clone(),
                            max_duration,
                        )
                    })?
            } else {
                inner.await
            };

            match result {
                Ok(result) => result,
                Err(unwind_error) => {
                    error!(
                        worker.name = self.inner.name,
                        "Worker panicked while handling a job: {unwind_error:?}"
                    );
                    Err(crate::error::worker::WorkerError::Panic(self.inner.name.clone()).into())
                }
            }
        }
        .instrument(span.or_current())
        .await
    }
}

#[derive(bon::Builder)]
#[non_exhaustive]
pub struct PeriodicArgs<Args>
where
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    pub args: Args,
    pub schedule: Schedule,
}

#[derive(Clone, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
pub(crate) struct PeriodicArgsJson {
    pub(crate) args: serde_json::Value,
    pub(crate) worker_name: String,
    pub(crate) schedule: Schedule,
}

#[cfg(any(feature = "worker-pg", feature = "worker-sidekiq"))]
impl std::hash::Hash for PeriodicArgsJson {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        crate::worker::job::periodic_hash(state, &self.worker_name, &self.schedule, &self.args);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use crate::worker::{Enqueuer, Worker};
    use async_trait::async_trait;
    use axum_core::extract::FromRef;
    use insta::assert_debug_snapshot;
    use rstest::{fixture, rstest};
    use serde::{Deserialize, Serialize};
    use std::borrow::Borrow;
    use std::time::Duration;

    struct FooBackend;

    #[async_trait]
    impl Enqueuer for FooBackend {
        type Error = crate::error::Error;

        async fn enqueue<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: ArgsRef,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }

        async fn enqueue_delayed<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: ArgsRef,
            _delay: Duration,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }

        async fn enqueue_batch<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: &[ArgsRef],
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }

        async fn enqueue_batch_delayed<W, S, Args, ArgsRef, E>(
            _state: &S,
            _args: &[ArgsRef],
            _delay: Duration,
        ) -> Result<(), Self::Error>
        where
            W: 'static + Worker<S, Args, Error = E>,
            S: Clone + Send + Sync + 'static,
            AppContext: FromRef<S>,
            Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
            ArgsRef: Send + Sync + Borrow<Args> + Serialize,
        {
            unimplemented!()
        }
    }

    #[derive(Serialize, Deserialize)]
    struct FooWorkerArgs {
        foo: String,
    }

    struct FooWorker;

    #[async_trait::async_trait]
    impl super::Worker<AppContext, FooWorkerArgs> for FooWorker {
        type Error = crate::error::Error;
        type Enqueuer = FooBackend;

        #[cfg_attr(coverage_nightly, coverage(off))]
        async fn handle(
            &self,
            _state: &AppContext,
            _args: FooWorkerArgs,
        ) -> Result<(), Self::Error> {
            unimplemented!()
        }
    }

    #[fixture]
    #[once]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn context() -> AppContext {
        let config = AppConfig::test(None).unwrap();
        AppContext::test(Some(config), None, None).unwrap()
    }

    #[rstest]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn enqueue_config(context: &AppContext) {
        let enqueue_config = FooWorker::enqueue_config(context);
        assert_debug_snapshot!(enqueue_config);
    }
}
