use crate::api::core::health::redis_health;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::health_check::{CheckResponse, HealthCheck};
use anyhow::anyhow;
use async_trait::async_trait;
use tracing::instrument;

pub struct SidekiqFetchHealthCheck {
    pub(crate) context: AppContext,
}

#[async_trait]
impl HealthCheck for SidekiqFetchHealthCheck {
    fn name(&self) -> String {
        "sidekiq-fetch".to_string()
    }

    fn enabled(&self) -> bool {
        enabled(&self.context)
    }

    #[instrument(skip_all)]
    async fn check(&self) -> RoadsterResult<CheckResponse> {
        Ok(redis_health(
            self.context
                .redis_fetch()
                .as_ref()
                .ok_or_else(|| anyhow!("Redis fetch connection pool is not present"))?,
            None,
        )
        .await)
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
