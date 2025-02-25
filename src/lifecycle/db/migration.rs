//! This [`AppLifecycleHandler`] runs the app's ['up' migrations][`crate::migration::Migrator::up`]
//! in [`AppLifecycleHandler::before_services`].

use crate::app::context::AppContext;
use crate::app::{App, PreparedAppWithoutCli};
use crate::error::RoadsterResult;
use crate::lifecycle::AppLifecycleHandler;
use crate::migration::UpArgs;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use tracing::instrument;

pub struct DbMigrationLifecycleHandler;

#[async_trait]
impl<A, S> AppLifecycleHandler<A, S> for DbMigrationLifecycleHandler
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn name(&self) -> String {
        "db-migration".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        context.config().database.auto_migrate
            && context
                .config()
                .lifecycle_handler
                .db_migration
                .common
                .enabled(&context)
    }

    fn priority(&self, state: &S) -> i32 {
        let context = AppContext::from_ref(state);
        context
            .config()
            .lifecycle_handler
            .db_migration
            .common
            .priority
    }

    #[instrument(skip_all)]
    async fn before_services(
        &self,
        prepared_app: &PreparedAppWithoutCli<A, S>,
    ) -> RoadsterResult<()> {
        for migrator in prepared_app.migrators.iter() {
            migrator
                .up(&prepared_app.state, &UpArgs::builder().build())
                .await?;
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
        config.lifecycle_handler.db_migration.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let handler = DbMigrationLifecycleHandler;

        // Act/Assert
        assert_eq!(
            AppLifecycleHandler::<MockApp<AppContext>, AppContext>::enabled(&handler, &context),
            expected_enabled
        );
    }

    #[rstest]
    #[case(None, 0)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config.lifecycle_handler.db_migration.common.priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let handler = DbMigrationLifecycleHandler;

        // Act/Assert
        assert_eq!(
            AppLifecycleHandler::<MockApp<AppContext>, AppContext>::priority(&handler, &context),
            expected_priority
        );
    }
}
