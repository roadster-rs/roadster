use crate::api::cli::CliState;
use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::App;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::service::HttpService;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::Parser;
use itertools::Itertools;
use serde_derive::Serialize;
use tabled::settings::themes::ColumnNames;
use tabled::settings::{Margin, Style};
use tabled::{Table, Tabled};
use tracing::info;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct ListRoutesArgs {}

#[derive(Tabled, bon::Builder)]
#[tabled(rename_all = "Upper Title Case")]
struct Route {
    #[builder(into)]
    method: String,
    #[builder(into)]
    path: String,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for ListRoutesArgs
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, cli: &CliState<A, S>) -> RoadsterResult<bool> {
        let routes = cli
            .service_registry
            .invoke(async |srvc: &HttpService| {
                srvc.list_routes()
                    .into_iter()
                    .map(|(path, method)| Route::builder().method(method).path(path).build())
                    .collect_vec()
            })
            .await?;

        let mut table = Table::builder(routes).build();
        table.with(Style::blank());
        table.with(Margin::new(4, 0, 1, 1));

        info!("API routes:\n{table}");

        Ok(true)
    }
}
