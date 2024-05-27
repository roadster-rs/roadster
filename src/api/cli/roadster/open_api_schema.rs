use clap::Parser;
use serde_derive::Serialize;
use std::path::PathBuf;

#[derive(Debug, Parser, Serialize)]
pub struct OpenApiArgs {
    /// The file to write the schema to. If not provided, will write to stdout.
    #[clap(short, long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub output: Option<PathBuf>,
    /// Whether to pretty-print the schema. Default: false.
    #[clap(short, long, default_value_t = false)]
    pub pretty_print: bool,
}
