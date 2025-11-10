use crate::api::core::health::redis_health;
use crate::app::context::{AppContext, AppContextWeak};
use crate::health::check::{CheckResponse, HealthCheck, missing_context_response};
use async_trait::async_trait;
use tracing::instrument;

pub struct SidekiqEnqueueHealthCheck {
    pub(crate) context: AppContextWeak,
}

#[async_trait]
impl HealthCheck for SidekiqEnqueueHealthCheck {
    type Error = crate::error::Error;

    fn name(&self) -> String {
        "sidekiq-enqueue".to_string()
    }

    fn enabled(&self) -> bool {
        self.context
            .upgrade()
            .map(|context| enabled(&context))
            .unwrap_or_default()
    }

    #[instrument(skip_all)]
    async fn check(&self) -> Result<CheckResponse, Self::Error> {
        let context = self.context.upgrade();
        let response = match context {
            Some(context) => redis_health(context.redis_enqueue(), None).await,
            None => missing_context_response(),
        };
        Ok(response)
    }
}

fn enabled(context: &AppContext) -> bool {
    context
        .config()
        .health_check
        .worker_sidekiq
        .common
        .enabled(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.health_check.default_enable = default_enable;
        config.health_check.worker_sidekiq.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        // Act/Assert
        assert_eq!(super::enabled(&context), expected_enabled);
    }
}
