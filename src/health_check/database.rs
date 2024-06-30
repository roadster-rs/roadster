use crate::api::core::health::{db_health, Status};
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::health_check::HealthCheck;
use anyhow::anyhow;
use async_trait::async_trait;
use axum::extract::FromRef;
use tracing::instrument;

pub struct DatabaseHealthCheck;

#[async_trait]
impl<S> HealthCheck<S> for DatabaseHealthCheck
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "db".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        enabled(&context)
    }

    #[instrument(skip_all)]
    async fn check(&self, state: &S) -> RoadsterResult<()> {
        let context = AppContext::from_ref(state);
        let health = db_health(&context, None).await;

        if let Status::Err(err) = health.status {
            return Err(anyhow!("Database connection pool is not healthy: {:?}", err).into());
        }

        Ok(())
    }
}

fn enabled(context: &AppContext) -> bool {
    context
        .config()
        .health_check
        .database
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
        config.health_check.database.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        // Act/Assert
        assert_eq!(super::enabled(&context), expected_enabled);
    }
}
