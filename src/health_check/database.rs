use crate::api::core::health::{db_health, Status};
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use crate::health_check::HealthCheck;
use anyhow::anyhow;
use async_trait::async_trait;
use tracing::instrument;

pub struct DatabaseHealthCheck;

#[async_trait]
impl<A: App + 'static> HealthCheck<A> for DatabaseHealthCheck {
    fn name(&self) -> String {
        "db".to_string()
    }

    fn enabled(&self, context: &AppContext<A::State>) -> bool {
        context
            .config()
            .health_check
            .database
            .common
            .enabled(context)
    }

    #[instrument(skip_all)]
    async fn check(&self, app_context: &AppContext<A::State>) -> RoadsterResult<()> {
        let health = db_health(app_context, None).await;

        if let Status::Err(err) = health.status {
            return Err(anyhow!("Database connection pool is not healthy: {:?}", err).into());
        }

        Ok(())
    }
}
