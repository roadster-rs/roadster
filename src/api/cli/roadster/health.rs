use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::api::core::health::health_check;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use clap::Parser;
use serde_derive::Serialize;
use tracing::info;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct HealthArgs {}

#[async_trait]
impl<A> RunRoadsterCommand<A> for HealthArgs
where
    A: App,
{
    async fn run(
        &self,
        _app: &A,
        _cli: &RoadsterCli,
        #[allow(unused_variables)] context: &AppContext<A::State>,
    ) -> RoadsterResult<bool> {
        let health = health_check::<A::State>(
            #[cfg(any(feature = "sidekiq", feature = "db-sql"))]
            context,
        )
        .await?;
        let health = serde_json::to_string_pretty(&health)?;
        info!("\n{health}");
        Ok(true)
    }
}
