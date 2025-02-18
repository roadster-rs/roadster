use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::context::AppContext;
use crate::app::{App, PreparedApp};
use crate::error::RoadsterResult;
use crate::service::http::service::HttpService;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::Parser;
use itertools::Itertools;
use serde_derive::Serialize;
use tracing::info;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct ListRoutesArgs {}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for ListRoutesArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, prepared_app: &PreparedApp<A, S>) -> RoadsterResult<bool> {
        let http_service = prepared_app
            .service_registry
            .get::<HttpService>()
            .map_err(|err| {
                anyhow!("Unable to get HttpService from registry. Was it registered? Err: {err}")
            })?;

        let routes = http_service
            .list_routes()
            .into_iter()
            .map(|(path, method)| format!("[{method}]\t{path}"))
            .join("\n\t");

        info!("API routes:\n\t{routes}");

        Ok(true)
    }
}
