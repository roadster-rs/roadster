use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::any::{Any, type_name, type_name_of_val};
use std::collections::HashMap;
use std::ops::Neg;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use typed_builder::TypedBuilder;
use validator::Validate;

#[cfg(feature = "worker-pg")]
pub mod pg;
#[cfg(feature = "worker-sidekiq")]
pub mod sidekiq;

/// Worker configuration options. Default values for these options can be set via the app's
/// configuration files. The options can also be overridden on a per-worker basis by implementing
/// the [`Worker::config`] method.
///
/// The [`Worker::config`] method will be called once for each worker when it is registered, and
/// the config will be stored by the [`Processor`] to be used when the worker handles a job.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct WorkerConfig {
    /// The name of the queue used to enqueue jobs. Multiple workers can enqueue jobs on the same
    /// queue, which is particularly useful for workers that may not have many jobs. However,
    /// workers can also be configured to use a dedicated queue.
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub queue: Option<String>,

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
    type Error: std::error::Error;

    //  this will be encoded in the job data, so it needs to be
    //  resilient to refactoring. We also need to have a common place where
    //  the logic for creating this name lives.
    fn name() -> String
    where
        Self: Sized,
    {
        worker_name::<Self>()
    }

    fn config(&self, _state: &S) -> WorkerConfig {
        WorkerConfig::default()
    }

    async fn handle(&self, state: &S, args: &Args) -> Result<(), Self::Error>;

    async fn enqueue(state: &S, args: &Args) -> Result<(), Self::Error>
    where
        Self: Sized;

    async fn enqueue_delayed(state: &S, args: &Args, delay: Duration) -> Result<(), Self::Error>
    where
        Self: Sized;
}

pub fn worker_name<T>() -> String
where
{
    type_name::<T>()
        .split("::")
        .last()
        .unwrap_or(type_name::<T>())
        .to_owned()
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

#[derive(Serialize, Deserialize)]
struct JobMetadata {
    worker_name: String,
    args: String,
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
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
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
                        // Todo: better error handling
                        // todo: timeouts, etc
                        Err(err) => Err(anyhow!("foo").into()),
                    }
                })
            }),
        );
        Ok(())
    }

    // Todo: don't require a worker instance to enqueue
    // Todo: the `enqueue` method maybe shouldn't be on the Processor?
    // Todo: allow configuring the queue backend (sidekiq/faktory/pgmq/etc)
    async fn enqueue<W, Args, E>(&self, worker: W, args: Args) -> RoadsterResult<()>
    where
        // todo: can we get rid of the `'static`?
        W: 'static + Worker<S, Args, Error = E>,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        // let worker_name = Self::worker_name(&worker).to_string();
        // // Todo: allow the worker to configure the queue name
        // let queue_name = worker_name.clone();
        // let args = serde_json::to_string(&args)?;
        // let metadata = JobMetadata { worker_name, args };
        // let context = AppContext::from_ref(&self.state);
        // context.pgmq().send(&queue_name, &metadata).await?;
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

        async fn enqueue(state: &AppContext, args: &FooWorkerArgs) -> Result<(), Self::Error>
        where
            Self: Sized,
        {
            todo!()
        }

        async fn enqueue_delayed(
            state: &AppContext,
            args: &FooWorkerArgs,
            delay: Duration,
        ) -> Result<(), Self::Error>
        where
            Self: Sized,
        {
            todo!()
        }
    }

    #[test]
    fn worker_name() {
        let worker_name = super::worker_name::<FooWorker>();
        assert_debug_snapshot!(worker_name);
    }
}
