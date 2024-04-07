use async_trait::async_trait;
use clap::Parser;
use tracing::info;

use crate::app_context::AppContext;
use crate::cli::{RoadsterCli, RunRoadsterCommand};

#[derive(Debug, Parser)]
pub struct ListRoutesArgs {}

#[async_trait]
impl RunRoadsterCommand for ListRoutesArgs {
    async fn run(&self, _cli: &RoadsterCli, context: &AppContext) -> anyhow::Result<bool> {
        info!("API routes:");
        context
            .api
            .as_ref()
            .operations()
            .for_each(|(path, method, _operation)| info!("[{method}]\t{path}"));
        Ok(true)
    }
}
