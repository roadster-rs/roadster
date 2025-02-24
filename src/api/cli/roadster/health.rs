use crate::api::cli::roadster::RunRoadsterCommand;
use crate::api::cli::CliState;
use crate::api::core::health::health_check;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::Parser;
use serde_derive::Serialize;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct HealthArgs {
    /// Maximum time to spend checking the health of the resources in milliseconds
    #[clap(short = 'd', long)]
    max_duration: Option<u64>,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for HealthArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, cli: &CliState<A, S>) -> RoadsterResult<bool> {
        let duration = self
            .max_duration
            .map(Duration::from_millis)
            .unwrap_or_else(|| {
                let context = AppContext::from_ref(&cli.state);
                context.config().health_check.max_duration.cli
            });
        let health = health_check(&cli.state, Some(duration)).await?;
        let health = serde_json::to_string_pretty(&health)?;
        info!("\n{health}");
        Ok(true)
    }
}
