use crate::app::MyApp;
use async_trait::async_trait;
use clap::Parser;
use roadster::api::cli::{CliState, RunCommand};
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;

#[derive(Parser)]
pub struct Cli;

#[async_trait]
impl RunCommand<MyApp, AppContext> for Cli {
    async fn run(&self, _prepared_app: &CliState<MyApp, AppContext>) -> RoadsterResult<bool> {
        todo!()
    }
}
