use clap::Parser;
use serde_derive::Serialize;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct ListRoutesArgs {}
