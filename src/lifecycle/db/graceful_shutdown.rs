//! This [`AppLifecycleHandler`] closes the DB connection pool when the app is shutting down.

use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::lifecycle::AppLifecycleHandler;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use tracing::instrument;

pub struct DbGracefulShutdownLifecycleHandler;

#[async_trait]
impl<A, S> AppLifecycleHandler<A, S> for DbGracefulShutdownLifecycleHandler
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn name(&self) -> String {
        "db-graceful-shutdown".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        context
            .config()
            .lifecycle_handler
            .db_graceful_shutdown
            .common
            .enabled(&context)
    }

    fn priority(&self, state: &S) -> i32 {
        let context = AppContext::from_ref(state);
        context
            .config()
            .lifecycle_handler
            .db_graceful_shutdown
            .common
            .priority
    }

    #[instrument(skip_all)]
    async fn on_shutdown(&self, #[allow(unused_variables)] state: &S) -> RoadsterResult<()> {
        #[cfg(not(feature = "testing-mocks"))]
        {
            tracing::info!("Closing the DB connection pool.");
            let context = AppContext::from_ref(state);
            context.db().clone().close().await?;
        }
        #[cfg(feature = "testing-mocks")]
        {
            tracing::warn!("Unable to manually close the db connection pool when `testing-mocks` feature is enabled.");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::MockApp;
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
        config.lifecycle_handler.default_enable = default_enable;
        config.lifecycle_handler.db_graceful_shutdown.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let handler = DbGracefulShutdownLifecycleHandler;

        // Act/Assert
        assert_eq!(
            AppLifecycleHandler::<MockApp<AppContext>, AppContext>::enabled(&handler, &context),
            expected_enabled
        );
    }

    #[rstest]
    #[case(None, 10000)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .lifecycle_handler
                .db_graceful_shutdown
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let handler = DbGracefulShutdownLifecycleHandler;

        // Act/Assert
        assert_eq!(
            AppLifecycleHandler::<MockApp<AppContext>, AppContext>::priority(&handler, &context),
            expected_priority
        );
    }
}
