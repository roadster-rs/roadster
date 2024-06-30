use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::api::core::health::health_check;
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum::extract::FromRef;
use clap::Parser;
use serde_derive::Serialize;
use tracing::info;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct HealthArgs {}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for HealthArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(
        &self,
        _app: &A,
        _cli: &RoadsterCli,
        #[allow(unused_variables)] state: &S,
    ) -> RoadsterResult<bool> {
        let health = health_check(state).await?;
        let health = serde_json::to_string_pretty(&health)?;
        info!("\n{health}");
        Ok(true)
    }
}
