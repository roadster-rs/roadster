use crate::app::MyApp;
use clap::Parser;
use roadster::api::cli::RunCommand;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use sea_orm::prelude::async_trait::async_trait;

#[derive(Parser)]
pub struct Cli;

#[async_trait]
impl RunCommand<MyApp, AppContext> for Cli {
    async fn run(&self, app: &MyApp, cli: &Cli, state: &AppContext) -> RoadsterResult<bool> {
        todo!()
    }
}
