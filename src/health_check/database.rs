use crate::api::core::health::db_health;
use crate::app::context::{AppContext, AppContextWeak};
use crate::error::RoadsterResult;
use crate::health_check::{missing_context_response, CheckResponse, HealthCheck};
use async_trait::async_trait;
use tracing::instrument;

pub struct DatabaseHealthCheck {
    pub(crate) context: AppContextWeak,
}

#[async_trait]
impl HealthCheck for DatabaseHealthCheck {
    fn name(&self) -> String {
        "db".to_string()
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
            Some(context) => db_health(&context, None).await,
            None => missing_context_response(),
        };
        Ok(response)
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
        config.health_check.database.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        // Act/Assert
        assert_eq!(super::enabled(&context), expected_enabled);
    }
}
