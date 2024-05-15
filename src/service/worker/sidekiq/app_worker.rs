use crate::app::App;
#[mockall_double::double]
use crate::app_context::AppContext;
use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use sidekiq::Worker;
use std::time::Duration;
use typed_builder::TypedBuilder;

/// Additional configuration options that can be configured via the app's configuration files.
/// The options can also be overridden on a per-worker basis by implementing the corresponding
/// method in the [AppWorker] trait.
#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[serde(default, rename_all = "kebab-case")]
pub struct AppWorkerConfig {
    /// The maximum number of times a job should be retried on failure.
    #[builder(default = AppWorkerConfig::default().max_retries)]
    pub max_retries: usize,
    /// True if Roadster should enforce a timeout on the app's workers. The default duration of
    /// the timeout can be configured with the `max-duration` option.
    #[builder(default = AppWorkerConfig::default().timeout)]
    pub timeout: bool,
    /// The maximum duration workers should run for. The timeout is only enforced if `timeout`
    /// is `true`.
    #[serde_as(as = "serde_with::DurationSeconds")]
    #[builder(default = AppWorkerConfig::default().max_duration)]
    pub max_duration: Duration,
    /// See <https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html#method.disable_argument_coercion>
    #[builder(default = AppWorkerConfig::default().disable_argument_coercion)]
    pub disable_argument_coercion: bool,
}

impl Default for AppWorkerConfig {
    fn default() -> Self {
        AppWorkerConfig::builder()
            .max_retries(5)
            .timeout(true)
            .max_duration(Duration::from_secs(60))
            .disable_argument_coercion(false)
            .build()
    }
}

#[async_trait]
pub trait AppWorker<A, Args>: Worker<Args>
where
    Self: Sized,
    A: App,
    Args: Send + Sync + serde::Serialize + 'static,
{
    /// Build a new instance of the [worker][Self].
    fn build(context: &AppContext<A::State>) -> Self;

    /// Enqueue the worker into its Sidekiq queue. This is a helper method around [Worker::perform_async]
    /// so the caller can simply provide the [state][App::State] instead of needing to access the
    /// [sidekiq::RedisPool] from inside the [state][App::State].
    async fn enqueue(context: &AppContext<A::State>, args: Args) -> anyhow::Result<()> {
        Self::perform_async(context.redis_enqueue(), args).await?;
        Ok(())
    }

    /// Provide the [AppWorkerConfig] for [Self]. The default implementation populates the
    /// [AppWorkerConfig] using the values from the corresponding methods on [Self], e.g.,
    /// [Self::max_retries].
    fn config(&self, context: &AppContext<A::State>) -> AppWorkerConfig {
        AppWorkerConfig::builder()
            .max_retries(AppWorker::max_retries(self, context))
            .timeout(self.timeout(context))
            .max_duration(self.max_duration(context))
            .disable_argument_coercion(AppWorker::disable_argument_coercion(self, context))
            .build()
    }

    /// See [AppWorkerConfig::max_retries].
    ///
    /// The default implementation uses the value from the app's config file.
    fn max_retries(&self, context: &AppContext<A::State>) -> usize {
        context
            .config()
            .service
            .sidekiq
            .custom
            .worker_config
            .max_retries
    }

    /// See [AppWorkerConfig::timeout].
    ///
    /// The default implementation uses the value from the app's config file.
    fn timeout(&self, context: &AppContext<A::State>) -> bool {
        context
            .config()
            .service
            .sidekiq
            .custom
            .worker_config
            .timeout
    }

    /// See [AppWorkerConfig::max_duration].
    ///
    /// The default implementation uses the value from the app's config file.
    fn max_duration(&self, context: &AppContext<A::State>) -> Duration {
        context
            .config()
            .service
            .sidekiq
            .custom
            .worker_config
            .max_duration
    }

    /// See [AppWorkerConfig::disable_argument_coercion].
    ///
    /// The default implementation uses the value from the app's config file.
    fn disable_argument_coercion(&self, context: &AppContext<A::State>) -> bool {
        context
            .config()
            .service
            .sidekiq
            .custom
            .worker_config
            .disable_argument_coercion
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[derive(Debug, Deserialize, Serialize)]
    struct Wrapper<T> {
        inner: T,
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_config_override_max_retries() {
        let max_retries = 1234;
        let value: Wrapper<AppWorkerConfig> = from_str(&format!(
            r#"{{"inner": {{"max-retries": {max_retries} }} }}"#
        ))
        .unwrap();
        assert_eq!(value.inner.max_retries, max_retries);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_config_override_timeout() {
        let value: Wrapper<AppWorkerConfig> =
            from_str(r#"{"inner": {"timeout": false } }"#).unwrap();
        assert!(!value.inner.timeout);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_config_override_max_duration() {
        let max_duration = Duration::from_secs(1234);
        let value: Wrapper<AppWorkerConfig> = from_str(&format!(
            r#"{{"inner": {{"max-duration": {} }} }}"#,
            max_duration.as_secs()
        ))
        .unwrap();
        assert_eq!(value.inner.max_duration, max_duration);
    }

    #[test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_config_override_disable_argument_coercion() {
        let value: Wrapper<AppWorkerConfig> =
            from_str(r#"{"inner": {"disable-argument-coercion": true } }"#).unwrap();
        assert!(value.inner.disable_argument_coercion);
    }
}
