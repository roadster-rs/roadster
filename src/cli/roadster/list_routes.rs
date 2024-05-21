use clap::Parser;
use serde_derive::Serialize;

#[derive(Debug, Parser, Serialize)]
pub struct ListRoutesArgs {}
