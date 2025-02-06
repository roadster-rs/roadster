#![allow(clippy::disallowed_macros)]

use cargo_manifest::Manifest;
use clap::Parser;
use itertools::Itertools;
use powerset_matrix::cli::{Cli, Format};
use powerset_matrix::powerset;
use serde_derive::Serialize;
use std::collections::BTreeSet;
use typed_builder::TypedBuilder;

#[derive(Debug, Serialize, TypedBuilder)]
struct Output {
    indexes: Vec<usize>,
    powersets: Vec<Vec<String>>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let manifest = Manifest::from_path("../../Cargo.toml")?;

    // Features to skip -- helps reduce the size of the powerset
    let skip = vec!["default"]
        .into_iter()
        .map(|f| f.to_string())
        .collect_vec();

    // Features to group together -- helps reduce the size of the powerset
    let groups: Vec<BTreeSet<String>> = vec![
        vec!["email", "email-smtp"],
        vec!["email", "email-sendgrid"],
        vec!["jwt", "jwt-ietf"],
        vec!["jwt", "jwt-openid"],
        vec!["open-api", "http"],
        vec!["db-sea-orm", "db-sql"],
    ]
    .into_iter()
    .map(|v| v.into_iter().map(|s| s.to_string()).collect())
    .collect_vec();

    let powerset = powerset(&cli, manifest, groups, skip)?;
    let total_powersets = powerset.len();
    let powerset = powerset.into_iter().map(|v| v.join(",")).collect_vec();

    let powersets = if let Some(group_size) = cli.group_size {
        let groups = powerset
            .chunks(group_size)
            .map(|v| v.to_vec())
            .collect_vec();
        groups
    } else {
        vec![powerset]
    };
    let indexes = (0..powersets.len()).collect_vec();
    let output = Output::builder()
        .indexes(indexes)
        .powersets(powersets)
        .build();

    match cli.format {
        Format::Debug => {
            println!("{output:?}");
        }
        Format::Json => {
            println!("{}", serde_json::to_string(&output)?);
        }
        Format::JsonPretty => {
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            unimplemented!()
        }
    }

    eprintln!("Total powersets: {}", total_powersets);
    Ok(())
}
