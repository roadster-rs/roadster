use crate::api::core::health::redis_health;
use crate::app::context::{AppContext, AppContextWeak};
use crate::error::RoadsterResult;
use crate::health::check::{CheckResponse, HealthCheck, missing_context_response};
use async_trait::async_trait;
use tracing::instrument;

pub struct SidekiqFetchHealthCheck {
    pub(crate) context: AppContextWeak,
}

#[async_trait]
impl HealthCheck for SidekiqFetchHealthCheck {
    fn name(&self) -> String {
        "sidekiq-fetch".to_string()
    }

    fn enabled(&self) -> bool {
        self.context
            .upgrade()
            .map(|context| enabled(&context))
            .unwrap_or_default()
    }

    #[instrument(skip_all)]
    async fn check(&self) -> RoadsterResult<CheckResponse> {
        let context = self.context.upgrade();
        let response = match context {
            Some(context) => {
                let redis = context.redis_fetch().as_ref().ok_or_else(|| {
                    crate::error::sidekiq::SidekiqError::Message(
                        "Redis fetch connection pool is not present".to_owned(),
                    )
                })?;
                redis_health(redis, None).await
            }
            None => missing_context_response(),
        };
        Ok(response)
    }
}

fn enabled(context: &AppContext) -> bool {
    context.redis_fetch().is_some()
        && context
            .config()
            .health_check
            .sidekiq
            .common
            .enabled(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use bb8::Pool;
    use rstest::rstest;
    use sidekiq::RedisConnectionManager;

    #[rstest]
    #[case(false, Some(true), true, true)]
    #[case(false, Some(true), false, false)]
    #[case(false, Some(false), false, false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    #[tokio::test]
    async fn enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] pool: bool,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.health_check.default_enable = default_enable;
        config.health_check.sidekiq.common.enable = enable;

        let redis_fetch_pool = if pool {
            let redis_fetch = RedisConnectionManager::new("redis://invalid_host:1234").unwrap();
            let pool = Pool::builder().build_unchecked(redis_fetch);
            Some(pool)
        } else {
            None
        };
        let context = AppContext::test(Some(config), None, redis_fetch_pool).unwrap();

        // Act/Assert
        assert_eq!(super::enabled(&context), expected_enabled);
    }
}
