use crate::api::core::health::db_diesel_health;
use crate::app::context::{AppContext, AppContextWeak};
use crate::health::check::{CheckResponse, HealthCheck, missing_context_response};
use async_trait::async_trait;
use tracing::instrument;

pub struct DbDieselHealthCheck<C, F>
where
    C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
    F: Send + Sync + Fn(&AppContext) -> &r2d2::Pool<diesel::r2d2::ConnectionManager<C>>,
{
    context: AppContextWeak,
    name: String,
    get_pool: F,
}

impl<C, F> DbDieselHealthCheck<C, F>
where
    C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
    F: Send + Sync + Fn(&AppContext) -> &r2d2::Pool<diesel::r2d2::ConnectionManager<C>>,
{
    #[cfg_attr(
        not(any(
            feature = "db-diesel-postgres-pool",
            feature = "db-diesel-mysql-pool",
            feature = "db-diesel-sqlite-pool"
        )),
        allow(unused)
    )]
    pub(crate) fn new(context: &AppContext, name: impl ToString, get_pool: F) -> Self {
        Self {
            context: context.downgrade(),
            name: name.to_string(),
            get_pool,
        }
    }
}

#[async_trait]
impl<C, F> HealthCheck for DbDieselHealthCheck<C, F>
where
    C: 'static + diesel::connection::Connection + diesel::r2d2::R2D2Connection,
    F: Send + Sync + Fn(&AppContext) -> &r2d2::Pool<diesel::r2d2::ConnectionManager<C>>,
{
    type Error = crate::error::Error;

    fn name(&self) -> String {
        format!("db-diesel-{}", self.name)
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
            Some(context) => db_diesel_health((self.get_pool)(&context), None).await,
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
