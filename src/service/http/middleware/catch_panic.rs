#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use tower_http::catch_panic::CatchPanicLayer;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CatchPanicConfig {}

pub struct CatchPanicMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for CatchPanicMiddleware {
    fn name(&self) -> String {
        "catch-panic".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .catch_panic
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
            .catch_panic
            .common
            .priority
    }

    fn install(&self, router: Router, _context: &AppContext<S>) -> anyhow::Result<Router> {
        let router = router.layer(CatchPanicLayer::new());

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    fn enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .middleware
            .catch_panic
            .common
            .enable = enable;

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = CatchPanicMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 0)]
    #[case(Some(1234), 1234)]
    fn priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .catch_panic
                .common
                .priority = priority;
        }

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = CatchPanicMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
