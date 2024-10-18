use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::Parser;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};
use tracing::info;

use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::app::context::AppContext;
use crate::app::App;
use crate::error::RoadsterResult;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct PrintConfigArgs {
    /// Print the config with the specified format.
    #[clap(short, long, default_value = "debug")]
    pub format: Format,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case", tag = "type")]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    Debug,
    Json,
    JsonPretty,
    Toml,
    TomlPretty,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for PrintConfigArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, _app: &A, _cli: &RoadsterCli, state: &S) -> RoadsterResult<bool> {
        let context = AppContext::from_ref(state);
        match self.format {
            Format::Debug => {
                info!("\n{:?}", context.config())
            }
            Format::Json => {
                info!("\n{}", serde_json::to_string(&context.config())?)
            }
            Format::JsonPretty => {
                info!("\n{}", serde_json::to_string_pretty(&context.config())?)
            }
            Format::Toml => {
                info!("\n{}", toml::to_string(&context.config())?)
            }
            Format::TomlPretty => {
                info!("\n{}", toml::to_string_pretty(&context.config())?)
            }
        }

        Ok(true)
    }
}
