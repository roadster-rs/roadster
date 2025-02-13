use crate::api::cli::roadster::RunRoadsterCommand;
use crate::app::context::AppContext;
use crate::app::{App, PreparedApp};
use crate::error::RoadsterResult;
use crate::service::http::service::HttpService;
use anyhow::anyhow;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use std::path::PathBuf;

#[derive(Debug, serde_derive::Serialize, typed_builder::TypedBuilder)]
#[cfg_attr(feature = "cli", derive(clap::Parser))]
#[non_exhaustive]
pub struct OpenApiArgs {
    /// The file to write the schema to. If not provided, will write to stdout.
    #[builder(default, setter(strip_option))]
    #[cfg_attr(feature = "cli", clap(short, long, value_name = "FILE", value_hint = clap::ValueHint::FilePath))]
    pub output: Option<PathBuf>,

    /// Whether to pretty-print the schema. Default: false.
    #[cfg_attr(feature = "cli", clap(short, long, default_value_t = false))]
    #[builder(default)]
    pub pretty_print: bool,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for OpenApiArgs
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
                anyhow!("Unable to get HttpService from registry. Was it registered?")
            })?;

        http_service.print_open_api_schema(self)?;

        Ok(true)
    }
}
