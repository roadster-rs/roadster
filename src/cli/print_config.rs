use async_trait::async_trait;
use clap::Parser;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};
use tracing::info;

use crate::app::App;
use crate::app_context::AppContext;
use crate::cli::{RoadsterCli, RunRoadsterCommand};

#[derive(Debug, Parser)]
pub struct PrintConfigArgs {
    /// Print the config with the specified format.
    #[clap(short, long, default_value = "debug")]
    pub format: Format,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Format {
    Debug,
    Json,
    JsonPretty,
    Toml,
    TomlPretty,
}

#[async_trait]
impl<A> RunRoadsterCommand<A> for PrintConfigArgs
where
    A: App,
{
    async fn run(
        &self,
        _app: &A,
        _cli: &RoadsterCli,
        context: &AppContext,
    ) -> anyhow::Result<bool> {
        match self.format {
            Format::Debug => {
                info!("{:?}", context.config)
            }
            Format::Json => {
                info!("{}", serde_json::to_string(&context.config)?)
            }
            Format::JsonPretty => {
                info!("{}", serde_json::to_string_pretty(&context.config)?)
            }
            Format::Toml => {
                info!("{}", toml::to_string(&context.config)?)
            }
            Format::TomlPretty => {
                info!("{}", toml::to_string_pretty(&context.config)?)
            }
        }

        Ok(true)
    }
}