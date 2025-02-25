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
        // Diesel
        vec!["db-diesel", "db-sql"],
        vec!["db-diesel-postgres", "db-diesel", "db-sql"],
        vec!["db-diesel-postgres", "db-sql"],
        vec!["db-diesel-mysql", "db-diesel", "db-sql"],
        vec!["db-diesel-mysql", "db-sql"],
        vec!["db-diesel-sqlite", "db-diesel", "db-sql"],
        vec!["db-diesel-sqlite", "db-sql"],
        // Diesel pool
        vec![
            "db-diesel-postgres-pool",
            "db-diesel-postgres",
            "db-diesel",
            "db-sql",
        ],
        vec!["db-diesel-postgres-pool", "db-sql"],
        vec!["db-diesel-postgres-pool", "db-diesel"],
        vec!["db-diesel-postgres-pool", "db-diesel-postgres"],
        vec![
            "db-diesel-mysql-pool",
            "db-diesel-mysql",
            "db-diesel",
            "db-sql",
        ],
        vec!["db-diesel-mysql-pool", "db-sql"],
        vec!["db-diesel-mysql-pool", "db-diesel"],
        vec!["db-diesel-mysql-pool", "db-diesel-mysql"],
        vec![
            "db-diesel-sqlite-pool",
            "db-diesel-sqlite",
            "db-diesel",
            "db-sql",
        ],
        vec!["db-diesel-sqlite-pool", "db-sql"],
        vec!["db-diesel-sqlite-pool", "db-diesel"],
        vec!["db-diesel-sqlite-pool", "db-diesel-sqlite"],
        // Diesel async pool
        vec!["db-diesel-pool-async", "db-diesel", "db-sql"],
        vec![
            "db-diesel-postgres-pool-async",
            "db-diesel-pool-async",
            "db-diesel-postgres",
            "db-diesel",
            "db-sql",
        ],
        vec!["db-diesel-pool-async", "db-sql"],
        vec!["db-diesel-postgres-pool-async", "db-sql"],
        vec!["db-diesel-postgres-pool-async", "db-diesel"],
        vec!["db-diesel-postgres-pool-async", "db-diesel-postgres"],
        vec!["db-diesel-postgres-pool-async", "db-diesel-pool-async"],
        vec!["db-diesel-pool-async", "db-sql"],
        vec!["db-diesel-pool-async", "db-diesel"],
        vec!["db-diesel-pool-async", "db-diesel-postgres"],
        vec![
            "db-diesel-mysql-pool-async",
            "db-diesel-pool-async",
            "db-diesel-mysql",
            "db-diesel",
            "db-sql",
        ],
        vec!["db-diesel-pool-async", "db-sql"],
        vec!["db-diesel-mysql-pool-async", "db-sql"],
        vec!["db-diesel-mysql-pool-async", "db-diesel"],
        vec!["db-diesel-mysql-pool-async", "db-diesel-mysql"],
        vec!["db-diesel-mysql-pool-async", "db-diesel-pool-async"],
        vec!["db-diesel-pool-async", "db-sql"],
        vec!["db-diesel-pool-async", "db-diesel"],
        vec!["db-diesel-pool-async", "db-diesel-mysql"],
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
