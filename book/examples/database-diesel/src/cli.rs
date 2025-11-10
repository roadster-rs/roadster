use crate::app::MyApp;
use async_trait::async_trait;
use clap::Parser;
use roadster::api::cli::{CliState, RunCommand};
use roadster::app::context::AppContext;
use std::convert::Infallible;

#[derive(Parser)]
pub struct Cli;

#[async_trait]
impl RunCommand<MyApp, AppContext> for Cli {
    type Error = Infallible;

    async fn run(&self, _prepared_app: &CliState<MyApp, AppContext>) -> Result<bool, Self::Error> {
        todo!()
    }
}
