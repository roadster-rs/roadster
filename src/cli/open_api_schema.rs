use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use async_trait::async_trait;
use clap::Parser;
use tracing::info;

use crate::app_context::AppContext;
use crate::cli::{RoadsterCli, RunRoadsterCommand};

#[derive(Debug, Parser)]
pub struct OpenApiArgs {
    /// The file to write the schema to. If not provided, will write to stdout.
    #[clap(short, long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub output: Option<PathBuf>,
    /// Whether to pretty-print the schema. Default: false.
    #[clap(short, long, default_value_t = false)]
    pub pretty_print: bool,
}

#[async_trait]
impl RunRoadsterCommand for OpenApiArgs {
    async fn run(&self, _cli: &RoadsterCli, context: &AppContext) -> anyhow::Result<bool> {
        let schema_json = if self.pretty_print {
            serde_json::to_string_pretty(context.api.as_ref())?
        } else {
            serde_json::to_string(context.api.as_ref())?
        };
        if let Some(path) = &self.output {
            info!("Writing schema to {:?}", path);
            write!(File::create(path)?, "{schema_json}")?;
        } else {
            info!("OpenAPI schema:");
            info!("{schema_json}");
        };

        Ok(true)
    }
}
