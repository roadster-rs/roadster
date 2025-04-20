use crate::api::cli::CliState;
use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::App;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::service::{HttpService, OpenApiArgs};
use async_trait::async_trait;
use axum_core::extract::FromRef;

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for OpenApiArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, cli: &CliState<A, S>) -> RoadsterResult<bool> {
        let http_service = cli.service_registry.get::<HttpService>()?;

        http_service.print_open_api_schema(self)?;

        Ok(true)
    }
}
