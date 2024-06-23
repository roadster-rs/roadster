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
        context
            .config()
            .health_check
            .sidekiq
            .common
            .enabled(context)
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
