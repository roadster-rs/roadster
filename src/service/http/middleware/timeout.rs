use crate::app_context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct TimeoutConfig {
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }
}

pub struct TimeoutMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for TimeoutMiddleware {
    fn name(&self) -> String {
        "timeout".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .timeout
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext<S>) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .timeout
            .common
            .priority
    }

    fn install(&self, router: Router, context: &AppContext<S>) -> RoadsterResult<Router> {
        let timeout = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .timeout
            .custom
            .timeout;

        let router = router.layer(TimeoutLayer::new(*timeout));

        Ok(router)
    }
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
    fn timeout_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config.service.http.custom.middleware.timeout.common.enable = enable;

        let context = AppContext::<()>::test(Some(config), None).unwrap();

        let middleware = TimeoutMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 0)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn timeout_priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .timeout
                .common
                .priority = priority;
        }

        let context = AppContext::<()>::test(Some(config), None).unwrap();

        let middleware = TimeoutMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
