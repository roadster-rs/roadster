use crate::api::core::health::{all_sidekiq_redis_health, Status};
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::health_check::HealthCheck;
use anyhow::anyhow;
use async_trait::async_trait;
use tracing::instrument;

pub struct SidekiqHealthCheck;

#[async_trait]
impl<A: App + 'static> HealthCheck<A> for SidekiqHealthCheck {
    fn name(&self) -> String {
        "sidekiq".to_string()
    }

    fn enabled(&self, context: &AppContext<A::State>) -> bool {
        enabled(context)
    }

    #[instrument(skip_all)]
    async fn check(&self, app_context: &AppContext<A::State>) -> RoadsterResult<()> {
        let (redis_enqueue, redis_fetch) = all_sidekiq_redis_health(app_context, None).await;

        if let Status::Err(err) = redis_enqueue.status {
            return Err(anyhow!(
                "Sidekiq redis enqueue connection pool is not healthy: {:?}",
                err
            )
            .into());
        }
        if let Some(redis_fetch) = redis_fetch {
            if let Status::Err(err) = redis_fetch.status {
                return Err(anyhow!(
                    "Sidekiq redis fetch connection pool is not healthy: {:?}",
                    err
                )
                .into());
            }
        }

        Ok(())
    }
}

fn enabled<S>(context: &AppContext<S>) -> bool {
    context
        .config()
        .health_check
        .sidekiq
        .common
        .enabled(context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::app_config::AppConfig;
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
        config.health_check.sidekiq.common.enable = enable;

        let context = AppContext::<()>::test(Some(config), None, None).unwrap();

        // Act/Assert
        assert_eq!(super::enabled(&context), expected_enabled);
    }
}
