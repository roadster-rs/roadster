use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::error::worker::EnqueueError;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::__private::ser::constrain;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::any::{Any, type_name, type_name_of_val};
use std::collections::HashMap;
use std::ops::Neg;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use typed_builder::TypedBuilder;
use validator::Validate;

mod enqueuer;
#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

/// Worker configuration options to use when enqueuing a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`Worker::enqueue_config`] method.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct EnqueueConfig {
    /// The name of the queue used to enqueue jobs. Multiple workers can enqueue jobs on the same
    /// queue, which is particularly useful for workers that may not have many jobs. However,
    /// workers can also be configured to use a dedicated queue.
    ///
    /// Note: when used with a Postgres backend with `pgmq`, this will be used in table names.
    /// Postgres generally has a length limit for table names, so care should be taken to ensure
    /// this queue name is not too long or else the queue name will be truncated when used
    /// with `pgmq`.
    #[serde(default)]
    #[builder(default, setter(strip_option(fallback = queue_opt)))]
    pub queue: Option<String>,

    /// The queue backend to use to enqueue the job.
    #[serde(default)]
    #[builder(default, setter(strip_option(fallback = backend_opt)))]
    pub backend: Option<QueueBackend>,
}

/// Supported queue backends.
// todo: add a trait to allow consumers to extend?
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum QueueBackend {
    #[cfg(feature = "worker-sidekiq")]
    Sidekiq,
    #[cfg(feature = "worker-pg")]
    Pg,
}

/// Worker configuration options to use when handling a job. Default values for these options can
/// be set via the app's configuration files. The options can also be overridden on a per-worker
/// basis by implementing the [`Worker::worker_config`] method.
///
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig {
    /// The maximum number of times a job should be retried on failure.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub max_retries: Option<usize>,

    /// True if Roadster should enforce a timeout on the app's workers. The default duration of
    /// the timeout can be configured with the `max-duration` option.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub timeout: Option<bool>,

    /// The maximum duration workers should run for. The timeout is only enforced if `timeout`
    /// is `true`.
    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    #[builder(default, setter(strip_option))]
    pub max_duration: Option<Duration>,
}

#[async_trait]
pub trait Worker<S, Args>: Send + Sync
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    type Error: std::error::Error + Send + Sync;

    /// The name of the worker. This will be encoded in the job data when it's enqueued the backing
    /// database (Redis/Postgres), and used to identify which type should handle a job when it's
    /// fetched from the queue. Therefore, it should be unique across the app, and care should be
    /// taken when refactoring.
    ///
    /// By default, [`Self::name`] returns the name of the type that implements the [`Worker`]
    /// trait. See [`simple_type_name`].
    ///
    /// This is not included in the [`EnqueueConfig`] because [`EnqueueConfig`] is included in
    /// the [`crate::config::AppConfig`] to allow defining defaults for the config values, but
    /// the name needs to be specified separately for each [`Worker`].
    fn name() -> String
    where
        Self: Sized,
    {
        simple_type_name::<Self>()
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

    async fn handle(&self, state: &S, args: &Args) -> Result<(), Self::Error>;
    //
    // async fn enqueue(state: &S, args: &Args) -> Result<(), Self::Error>
    // where
    //     Self: Sized,
    // {
    //     enqueue::<Self, S, Args, Self::Error>(state, args).await
    // }
    //
    // async fn enqueue_delayed(state: &S, args: &Args, delay: Duration) -> Result<(), Self::Error>
    // where
    //     Self: Sized;
}

/// Get the name of the type with its module prefix stripped.
pub fn simple_type_name<T>() -> String
where
{
    type_name::<T>()
        .split("::")
        .last()
        .unwrap_or(type_name::<T>())
        .to_owned()
}

#[derive(Serialize, Deserialize)]
struct Job {
    metadata: JobMetadata,
    // Todo: use `serde_json::Value` instead?
    args: String,
}

#[derive(Serialize, Deserialize)]
struct JobMetadata {
    worker_name: String,
}

/// Same as [`EnqueueConfig`], except that all the required fields are not [`Option`].
#[derive(Debug, TypedBuilder)]
#[non_exhaustive]
struct EnqueueConfigRequired {
    pub queue: String,
    pub backend: QueueBackend,
}

fn enqueue_config<W, S, Args, E>(state: &S) -> Result<EnqueueConfigRequired, EnqueueError>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let context = AppContext::from_ref(state);
    let worker_enqueue_config = W::enqueue_config(state);
    let enqueue_config = &context.config().service.worker.enqueue_config;

    let backend = if let Some(backend) = worker_enqueue_config.backend {
        backend
    } else if let Some(backend) = enqueue_config.backend.as_ref() {
        backend.to_owned()
    } else {
        let worker_name = W::name();
        error!(worker_name, "Unable to enqueue job, no backend configured");
        return Err(EnqueueError::NoBackend(worker_name).into());
    };

    let queue = if let Some(queue) = worker_enqueue_config.queue {
        queue
    } else if let Some(queue) = enqueue_config.queue.as_ref() {
        queue.to_owned()
    } else {
        let worker_name = W::name();
        error!(worker_name, "Unable to enqueue job, no queue configured");
        return Err(EnqueueError::NoQueue(worker_name).into());
    };

    Ok(EnqueueConfigRequired::builder()
        .backend(backend)
        .queue(queue)
        .build())
}

async fn enqueue<W, S, Args, E>(state: &S, args: &Args) -> RoadsterResult<()>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let enqueue_config = enqueue_config::<W, S, Args, E>(state)?;
    let worker_name = W::name();
    let context = AppContext::from_ref(state);

    match enqueue_config.backend {
        #[cfg(feature = "worker-sidekiq")]
        QueueBackend::Sidekiq => {
            ::sidekiq::perform_async(
                context.redis_enqueue(),
                worker_name,
                enqueue_config.queue,
                args,
            )
            .await?;
            debug!("Job enqueued");
        }
        #[cfg(feature = "worker-pg")]
        QueueBackend::Pg => {
            let args = serde_json::to_string(&args)?;
            let job = Job {
                metadata: JobMetadata { worker_name },
                args,
            };
            let id = context.pgmq().send(&enqueue_config.queue, &job).await?;
            debug!(id, "Job enqueued");
        }
    }

    Ok(())
}

async fn enqueue_delayed<W, S, Args, E>(
    state: &S,
    args: &Args,
    delay: Duration,
) -> RoadsterResult<()>
where
    W: 'static + Worker<S, Args, Error = E>,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let enqueue_config = enqueue_config::<W, S, Args, E>(state)?;
    let worker_name = W::name();
    let context = AppContext::from_ref(state);

    match enqueue_config.backend {
        #[cfg(feature = "worker-sidekiq")]
        QueueBackend::Sidekiq => {
            ::sidekiq::perform_in(
                context.redis_enqueue(),
                delay,
                worker_name,
                enqueue_config.queue,
                args,
            )
            .await?;
            debug!(delay = delay.as_secs(), "Job enqueued");
        }
        #[cfg(feature = "worker-pg")]
        QueueBackend::Pg => {
            let args = serde_json::to_string(&args)?;
            let job = Job {
                metadata: JobMetadata { worker_name },
                args,
            };
            let id = context
                .pgmq()
                .send_delay(&enqueue_config.queue, &job, delay.as_secs())
                .await?;
            debug!(id, delay = delay.as_secs(), "Job enqueued");
        }
    }

    Ok(())
}

type WorkerFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(&'a S, &'a str) -> Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<()>>>>,
>;

struct Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    state: S,
    workers: HashMap<String, WorkerFn<S>>,
}

impl<S> Processor<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn register<W, Args, E>(mut self, worker: W) -> RoadsterResult<()>
    where
        // todo: can we get rid of the `'static`?
        W: 'static + Worker<S, Args, Error = E>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
        E: std::error::Error + Send + Sync,
    {
        // todo: can we get rid of the `Arc` (and the `clones` below)?
        let worker = Arc::new(worker);
        self.workers.insert(
            W::name(),
            // Todo: instrument to allow recording spans/metrics
            Box::new(move |state: &S, args: &str| {
                let worker = worker.clone();
                Box::pin(async move {
                    let args: Args = serde_json::from_str(args)?;
                    match worker.clone().handle(&state, &args).await {
                        Ok(_) => Ok(()),
                        // todo: timeouts, etc
                        Err(err) => Err(crate::error::worker::WorkerError::Handle(
                            W::name(),
                            Box::new(err),
                        )
                        .into()),
                    }
                })
            }),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::service::worker::Worker;
    use insta::_macro_support::assert_snapshot;
    use insta::assert_debug_snapshot;
    use serde_derive::{Deserialize, Serialize};
    use std::time::Duration;

    #[derive(Serialize, Deserialize)]
    struct FooWorkerArgs {
        foo: String,
    }

    struct FooWorker;

    #[async_trait::async_trait]
    impl Worker<AppContext, FooWorkerArgs> for FooWorker {
        type Error = crate::error::Error;

        async fn handle(
            &self,
            state: &AppContext,
            args: &FooWorkerArgs,
        ) -> Result<(), Self::Error> {
            todo!()
        }
    }

    #[test]
    fn simple_type_name() {
        let simple_type_name = super::simple_type_name::<FooWorker>();
        assert_debug_snapshot!(simple_type_name);
    }
}
