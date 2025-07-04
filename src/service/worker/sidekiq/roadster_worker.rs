use crate::app::context::AppContext;
use crate::service::worker::sidekiq::app_worker::AppWorkerConfig;
use crate::service::worker::sidekiq::app_worker::DEFAULT_MAX_DURATION;
use async_trait::async_trait;
use serde::Serialize;
use sidekiq::{RedisPool, Worker, WorkerOpts};
use std::marker::PhantomData;
use std::time::Duration;
use tracing::{error, instrument};

/// Worker used by Roadster to wrap the consuming app's workers to add additional behavior. For
/// example, [RoadsterWorker] is by default configured to automatically abort the app's worker
/// when it exceeds a certain timeout.
pub(crate) struct RoadsterWorker<Args, W>
where
    Args: Send + Sync + Serialize + 'static,
    W: Worker<Args>,
{
    inner: W,
    inner_config: Option<AppWorkerConfig>,
    context: AppContext,
    _args: PhantomData<Args>,
}

impl<Args, W> RoadsterWorker<Args, W>
where
    Args: Send + Sync + Serialize,
    W: Worker<Args>,
{
    pub(crate) fn new(context: &AppContext, inner: W, config: Option<AppWorkerConfig>) -> Self {
        Self {
            inner,
            inner_config: config,
            context: context.clone(),
            _args: PhantomData,
        }
    }
}

#[async_trait]
impl<Args, W> Worker<Args> for RoadsterWorker<Args, W>
where
    Args: Send + Sync + Serialize,
    W: Worker<Args>,
{
    fn disable_argument_coercion(&self) -> bool {
        self.inner_config
            .as_ref()
            .and_then(|config| config.disable_argument_coercion)
            .unwrap_or_else(|| {
                self.context
                    .config()
                    .service
                    .worker
                    .worker_config
                    .sidekiq
                    .as_ref()
                    .and_then(|config| config.disable_argument_coercion)
                    .unwrap_or_else(|| W::disable_argument_coercion(&self.inner))
            })
    }

    fn opts() -> WorkerOpts<Args, Self>
    where
        Self: Sized,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly,
        // and this method is only used when enqueuing. Instead, Sidekiq.rs will use the
        // `W::opts` implementation directly.
        unimplemented!()
    }

    fn max_retries(&self) -> usize {
        self.inner_config
            .as_ref()
            .and_then(|config| config.max_retries)
            .unwrap_or_else(|| {
                self.context
                    .config()
                    .service
                    .worker
                    .worker_config
                    .retry_config
                    .as_ref()
                    .and_then(|config| config.max_retries)
                    .unwrap_or_else(|| W::max_retries(&self.inner))
            })
    }

    fn class_name() -> String
    where
        Self: Sized,
    {
        // This method is implemented because it's used both when registering the worker, and
        // when enqueuing a job. We forward the implementation to `W::classname` because that's
        // what Sidekiq.rs uses specifically. If we attempt to override this, our impl will be used
        // when registering the worker, but not when enqueuing a job, so the worker will not pick
        // up the jobs.
        W::class_name()
    }

    async fn perform_async(_redis: &RedisPool, _args: Args) -> sidekiq::Result<()>
    where
        Self: Sized,
        Args: Send + Sync + Serialize + 'static,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly.
        unimplemented!()
    }

    async fn perform_in(_redis: &RedisPool, _duration: Duration, _args: Args) -> sidekiq::Result<()>
    where
        Self: Sized,
        Args: Send + Sync + Serialize + 'static,
    {
        // This method not implemented because `RoadsterWorker` should not be enqueued directly.
        unimplemented!()
    }

    #[instrument(skip_all)]
    async fn perform(&self, args: Args) -> sidekiq::Result<()> {
        let inner = self.inner.perform(args);

        let timeout = self
            .inner_config
            .as_ref()
            .and_then(|config| config.timeout)
            .unwrap_or_else(|| {
                self.context
                    .config()
                    .service
                    .worker
                    .worker_config
                    .timeout
                    .unwrap_or_default()
            });

        if timeout {
            let max_duration = self
                .inner_config
                .as_ref()
                .and_then(|config| config.max_duration)
                .unwrap_or_else(|| {
                    self.context
                        .config()
                        .service
                        .worker
                        .worker_config
                        .max_duration
                        .unwrap_or(DEFAULT_MAX_DURATION)
                });

            tokio::time::timeout(max_duration, inner)
                .await
                .map_err(|err| {
                    error!(
                        worker = %W::class_name(),
                        max_duration = %max_duration.as_secs(),
                        %err,
                        "Worker timed out"
                    );
                    sidekiq::Error::Any(Box::new(err))
                })?
        } else {
            inner.await
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::app::context::AppContext;
//     use crate::config::AppConfig;
//     use crate::service::worker::sidekiq::app_worker::AppWorkerConfig;
//     use crate::service::worker::sidekiq::roadster_worker::RoadsterWorker;
//     use crate::testing::snapshot::TestCase;
//     use crate::worker::worker::SidekiqWorkerConfig;
//     use async_trait::async_trait;
//     use rstest::{fixture, rstest};
//     use sidekiq::Worker;
//     use std::time::Duration;
//     use tokio::time::sleep;
//     use typed_builder::TypedBuilder;
//
//     #[derive(Debug, TypedBuilder)]
//     struct ExampleWorker {
//         #[builder(default)]
//         disable_argument_coercion: bool,
//         #[builder(default)]
//         max_retries: usize,
//         #[builder(default)]
//         duration: Option<Duration>,
//     }
//
//     #[async_trait]
//     impl Worker<()> for ExampleWorker {
//         fn disable_argument_coercion(&self) -> bool {
//             self.disable_argument_coercion
//         }
//
//         fn max_retries(&self) -> usize {
//             self.max_retries
//         }
//
//         async fn perform(&self, _args: ()) -> sidekiq::Result<()> {
//             sleep(self.duration.unwrap_or_default()).await;
//             Ok(())
//         }
//     }
//
//     #[fixture]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn case() -> TestCase {
//         Default::default()
//     }
//
//     #[rstest]
//     #[case(None, None, false, false)]
//     #[case(None, None, true, true)]
//     #[case(None, Some(false), false, false)]
//     #[case(None, Some(true), false, true)]
//     #[case(None, Some(false), true, false)]
//     #[case(Some(false), None, false, false)]
//     #[case(Some(true), None, false, true)]
//     #[case(Some(false), None, true, false)]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn disable_argument_coercion(
//         _case: TestCase,
//         #[case] config_disable: Option<bool>,
//         #[case] worker_config_disable: Option<bool>,
//         #[case] worker_field_disable: bool,
//         #[case] expected: bool,
//     ) {
//         // Arrange
//         let mut config = AppConfig::test(None).unwrap();
//         config.service.worker.worker_config.sidekiq = Some(
//             SidekiqWorkerConfig::builder()
//                 .disable_argument_coercion_opt(config_disable)
//                 .build(),
//         );
//         let context = AppContext::test(Some(config), None, None).unwrap();
//
//         let app_worker_config = worker_config_disable.map(|disable| {
//             AppWorkerConfig::builder()
//                 .disable_argument_coercion(disable)
//                 .build()
//         });
//
//         let example_worker = ExampleWorker::builder()
//             .disable_argument_coercion(worker_field_disable)
//             .build();
//
//         let roadster_worker = RoadsterWorker::new(&context, example_worker, app_worker_config);
//
//         // Act
//         let disable_argument_coercion = roadster_worker.disable_argument_coercion();
//
//         // Assert
//         assert_eq!(disable_argument_coercion, expected);
//     }
//
//     #[rstest]
//     #[case(None, None, 0, 0)]
//     #[case(None, None, 1, 1)]
//     #[case(None, Some(1), 0, 1)]
//     #[case(Some(1), None, 0, 1)]
//     #[case(Some(1), None, 0, 1)]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn max_retries(
//         _case: TestCase,
//         #[case] config_max_retries: Option<usize>,
//         #[case] worker_config_max_retries: Option<usize>,
//         #[case] worker_field_max_retries: usize,
//         #[case] expected: usize,
//     ) {
//         // Arrange
//         let mut config = AppConfig::test(None).unwrap();
//         config.service.worker.worker_config.max_retries = config_max_retries;
//         let context = AppContext::test(Some(config), None, None).unwrap();
//
//         let app_worker_config = worker_config_max_retries
//             .map(|max_retries| AppWorkerConfig::builder().max_retries(max_retries).build());
//
//         let example_worker = ExampleWorker::builder()
//             .max_retries(worker_field_max_retries)
//             .build();
//
//         let roadster_worker = RoadsterWorker::new(&context, example_worker, app_worker_config);
//
//         // Act
//         let max_retries = roadster_worker.max_retries();
//
//         // Assert
//         assert_eq!(max_retries, expected);
//     }
//
//     #[test]
//     #[should_panic]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn opts() {
//         let _ = RoadsterWorker::<(), ExampleWorker>::opts();
//     }
//
//     #[test]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn class_name() {
//         assert_eq!(
//             RoadsterWorker::<(), ExampleWorker>::class_name(),
//             ExampleWorker::class_name()
//         );
//     }
//
//     #[tokio::test]
//     #[should_panic]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     async fn perform_async() {
//         let context = AppContext::test(None, None, None).unwrap();
//         let _ =
//             RoadsterWorker::<(), ExampleWorker>::perform_async(context.redis_enqueue(), ()).await;
//     }
//
//     #[tokio::test]
//     #[should_panic]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     async fn perform_in() {
//         let context = AppContext::test(None, None, None).unwrap();
//         let _ = RoadsterWorker::<(), ExampleWorker>::perform_in(
//             context.redis_enqueue(),
//             Duration::from_secs(1),
//             (),
//         )
//         .await;
//     }
//
//     #[rstest]
//     #[case(None, None, None, true)]
//     #[case(None, None, Some(Duration::from_secs(2)), true)]
//     #[case(None, Some(true), None, true)]
//     #[case(Some(true), None, None, true)]
//     #[case(None, Some(true), Some(Duration::from_secs(2)), false)]
//     #[case(Some(true), None, Some(Duration::from_secs(2)), false)]
//     #[case(Some(false), Some(true), Some(Duration::from_secs(2)), false)]
//     #[case(Some(true), Some(false), Some(Duration::from_secs(2)), true)]
//     #[tokio::test]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     async fn perform(
//         _case: TestCase,
//         #[case] config_timeout: Option<bool>,
//         #[case] worker_config_timeout: Option<bool>,
//         #[case] worker_duration: Option<Duration>,
//         #[case] success: bool,
//     ) {
//         // Arrange
//         let mut config = AppConfig::test(None).unwrap();
//         config.service.worker.worker_config.timeout = config_timeout;
//         config.service.worker.worker_config.max_duration = Some(Duration::from_secs(1));
//         let context = AppContext::test(Some(config), None, None).unwrap();
//
//         let app_worker_config = worker_config_timeout
//             .map(|timeout| AppWorkerConfig::builder().timeout(timeout).build());
//
//         let example_worker = ExampleWorker::builder().duration(worker_duration).build();
//
//         let roadster_worker = RoadsterWorker::new(&context, example_worker, app_worker_config);
//
//         // Act
//         let result = roadster_worker.perform(()).await;
//
//         // Assert
//         assert_eq!(result.is_ok(), success);
//     }
// }
