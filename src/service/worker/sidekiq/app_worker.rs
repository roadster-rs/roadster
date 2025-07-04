use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use sidekiq::Worker;
use std::time::Duration;
use typed_builder::TypedBuilder;
use validator::Validate;

pub(crate) const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(60);

/// Additional configuration options that can be configured via the app's configuration files.
/// The options can also be overridden on a per-worker basis by implementing the corresponding
/// method in the [AppWorker] trait.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
#[non_exhaustive]
pub struct AppWorkerConfig {
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

    /// See <https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html#method.disable_argument_coercion>
    #[serde(default)]
    #[builder(default, setter(strip_option))]
    pub disable_argument_coercion: Option<bool>,
}

#[async_trait]
pub trait AppWorker<S, Args>: Worker<Args>
where
    Self: Sized,
    Args: Send + Sync + serde::Serialize + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    /// Enqueue the worker into its Sidekiq queue. This is a helper method around [Worker::perform_async]
    /// so the caller can simply provide the app state instead of needing to access the
    /// [sidekiq::RedisPool] from inside the app state.
    async fn enqueue(state: &S, args: Args) -> RoadsterResult<()> {
        Self::perform_async(AppContext::from_ref(state).redis_enqueue(), args).await?;
        Ok(())
    }

    /// Enqueue the worker into its Sidekiq queue. This is a helper method around [Worker::perform_in]
    /// so the caller can simply provide the app state instead of needing to access the
    /// [sidekiq::RedisPool] from inside the app state.
    async fn enqueue_delayed(state: &S, delay: Duration, args: Args) -> RoadsterResult<()> {
        Self::perform_in(AppContext::from_ref(state).redis_enqueue(), delay, args).await?;
        Ok(())
    }
}

impl<S, Args, W> AppWorker<S, Args> for W
where
    Self: Sized,
    Args: Send + Sync + serde::Serialize + 'static,
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    W: Worker<Args>,
{
}
//
// #[cfg(test)]
// mod deserialize_tests {
//     use super::*;
//     use crate::testing::snapshot::TestCase;
//     use insta::assert_toml_snapshot;
//     use rstest::{fixture, rstest};
//
//     #[fixture]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn case() -> TestCase {
//         Default::default()
//     }
//
//     #[rstest]
//     #[case("")]
//     #[case(
//         r#"
//         max-retries = 1
//         "#
//     )]
//     #[case(
//         r#"
//         timeout = false
//         "#
//     )]
//     #[case(
//         r#"
//         timeout = true
//         "#
//     )]
//     #[case(
//         r#"
//         max-duration = 1234
//         "#
//     )]
//     #[case(
//         r#"
//         disable-argument-coercion = false
//         "#
//     )]
//     #[case(
//         r#"
//         disable-argument-coercion = true
//         "#
//     )]
//     #[cfg_attr(coverage_nightly, coverage(off))]
//     fn app_worker(_case: TestCase, #[case] config: &str) {
//         let app_worker: AppWorkerConfig = toml::from_str(config).unwrap();
//
//         assert_toml_snapshot!(app_worker);
//     }
// }
