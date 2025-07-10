use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::util::types;
use crate::worker::backend::sidekiq::roadster_worker::RoadsterWorker;
use crate::worker::config::{EnqueueConfig, WorkerConfig};
use crate::worker::enqueue::Enqueuer;
use crate::worker::job::periodic_hash;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use std::any::{Any, TypeId};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, instrument};

pub mod backend;
pub mod config;
pub(crate) mod enqueue;
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
    /// registered, and the config will be stored by the [`Processor`] to be used when the worker
    /// handles a job.
    fn worker_config(&self, _state: &S) -> WorkerConfig {
        WorkerConfig::default()
    }

    #[instrument(skip_all)]
    async fn enqueue<T>(state: &S, args: T) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
        T: Send + Sync + Borrow<Args> + Serialize,
    {
        Self::Enqueuer::enqueue::<Self, _, _, _, Self::Error>(state, args).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_delayed(
        state: &S,
        args: &Args,
        delay: Duration,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue_delayed::<Self, _, _, _, Self::Error>(state, args, delay).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch(
        state: &S,
        args: &[Args],
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue_batch::<Self, _, _, _, Self::Error>(state, args).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn enqueue_batch_delayed(
        state: &S,
        args: &[Args],
        delay: Duration,
    ) -> Result<(), <Self::Enqueuer as Enqueuer>::Error>
    where
        Self: 'static + Sized,
    {
        Self::Enqueuer::enqueue_batch_delayed::<Self, _, _, _, Self::Error>(state, args, delay)
            .await?;
        Ok(())
    }

    async fn handle(&self, state: &S, args: Args) -> Result<(), Self::Error>;
}

type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
            serde_json::Value,
        ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

type RegisterSidekiqFn<S> =
    Box<dyn for<'a> Fn(&'a S, &'a mut ::sidekiq::Processor, WorkerWrapper<S>)>;

// Returns the sidekiq json for the periodic job
type RegisterSidekiqPeriodicFn<S> = Box<
    dyn for<'a> Fn(
        &'a S,
        &'a mut ::sidekiq::Processor,
        WorkerWrapper<S>,
        PeriodicArgsJson,
    ) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<String>>>>,
>;

#[derive(Clone)]
pub(crate) struct WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    inner: Arc<WorkerWrappeInner<S>>,
}

pub(crate) struct WorkerWrappeInner<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    name: String,
    type_id: TypeId,
    #[allow(dead_code)]
    enqueue_config: EnqueueConfig,
    worker_config: WorkerConfig,
    worker_fn: WorkerFn<S>,
}

impl<S> WorkerWrapper<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn new<W, Args, E>(state: &S, worker: W, enqueue_config: EnqueueConfig) -> RoadsterResult<Self>
    where
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: 'static + std::error::Error + Send + Sync,
    {
        let worker = Arc::new(worker);

        Ok(Self {
            inner: Arc::new(WorkerWrappeInner {
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
            }),
        })
    }

    #[instrument(skip_all)]
    async fn handle(&self, state: &S, args: serde_json::Value) -> RoadsterResult<()> {
        let inner = (self.inner.worker_fn)(state, args);

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

        if let Some(max_duration) = max_duration {
            tokio::time::timeout(max_duration, inner)
                .await
                .map_err(|err| {
                    error!(
                        worker = self.inner.name,
                        max_duration = max_duration.as_secs(),
                        %err,
                        "Worker timed out"
                    );
                    crate::error::worker::WorkerError::Timeout(
                        self.inner.name.clone(),
                        max_duration,
                        Box::new(err),
                    )
                })?
        } else {
            inner.await
        }
    }
}

#[derive(bon::Builder)]
#[non_exhaustive]
pub struct PeriodicArgs<Args>
where
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    pub(crate) args: Args,
    pub(crate) schedule: Schedule,
}

#[derive(Clone, bon::Builder, Eq, PartialEq)]
#[non_exhaustive]
pub(crate) struct PeriodicArgsJson {
    pub(crate) args: serde_json::Value,
    pub(crate) worker_name: String,
    pub(crate) schedule: Schedule,
}

impl Hash for PeriodicArgsJson {
    fn hash<H: Hasher>(&self, state: &mut H) {
        periodic_hash(state, &self.worker_name, &self.schedule, &self.args);
    }
}
//
// impl Ord for PeriodicArgsJson {
//     fn cmp(&self, other: &Self) -> Ordering {
//         self.worker_name
//             .cmp(&other.worker_name)
//             .then(self.schedule.to_string().cmp(&other.schedule.to_string()))
//             .then(
//                 serde_json::to_string(&self.args)
//                     .unwrap_or_default()
//                     .cmp(&serde_json::to_string(&other.args).unwrap_or_default()),
//             )
//     }
// }
//
// impl PartialOrd for PeriodicArgsJson {
//     fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//         Some(self.cmp(other))
//     }
// }

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
            todo!()
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
            todo!()
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
            todo!()
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
            todo!()
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
            todo!()
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
