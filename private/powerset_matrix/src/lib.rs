#![allow(clippy::disallowed_macros)]

use crate::cli::Cli;
use anyhow::anyhow;
use cargo_manifest::Manifest;
use itertools::Itertools;
use rand::prelude::{IteratorRandom, StdRng};
use rand::SeedableRng;
use std::collections::BTreeSet;

pub mod cli;

pub fn powerset(
    cli: &Cli,
    manifest: Manifest,
    feature_groups: Vec<BTreeSet<String>>,
    features_to_skip: Vec<String>,
) -> anyhow::Result<Vec<Vec<String>>> {
    let features = manifest
        .features
        .into_iter()
        .flatten()
        .map(|f| f.0)
        .collect_vec();

    powerset_impl(cli, features, feature_groups, features_to_skip)
}

fn powerset_impl(
    cli: &Cli,
    features: Vec<String>,
    feature_groups: Vec<BTreeSet<String>>,
    features_to_skip: Vec<String>,
) -> anyhow::Result<Vec<Vec<String>>> {
    {
        let group_members = feature_groups
            .iter()
            .flat_map(|g| g.iter())
            .map(|f| f.to_string())
            .collect_vec();

        for feature in group_members {
            if !features.contains(&feature) {
                return Err(anyhow!(
                    "Group feature {feature} is not a valid feature name"
                ));
            }
        }
    }

    let features = features
        .into_iter()
        .filter(|f| !features_to_skip.contains(f))
        .collect_vec();

    let features = features
        .into_iter()
        .map(|n| {
            let group = feature_groups.iter().find(|g| g.contains(&n));
            if let Some(group) = group {
                group.iter().join(",")
            } else {
                n
            }
        })
        .unique()
        .collect_vec();

    let mut sets: Vec<Vec<&String>> = Default::default();

    // Start the powerset size at 2 because we already test each feature individually on each PR.
    for i in 1..cli.limited_depth {
        sets.extend(
            features
                .iter()
                .combinations(i + 1)
                .map(|x| x.into_iter().unique().collect_vec())
                .collect_vec(),
        );
    }

    let mut sets = sets
        .into_iter()
        .map(|x| x.iter().map(|x| x.to_string()).collect_vec())
        .collect_vec();

    sets.extend(random(cli, features)?);

    Ok(sets)
}

const RANDOM_MAX_DEPTH: usize = 6;

fn random(cli: &Cli, features: Vec<String>) -> anyhow::Result<Vec<Vec<String>>> {
    let count = if let Some(count) = cli.random_count {
        count
    } else {
        return Ok(vec![]);
    };

    let seed = if let Some(seed) = cli.random_seed {
        seed
    } else {
        rand::random()
    };
    println!("Foo");
    eprintln!("Using seed {seed}");
    let mut rng = StdRng::seed_from_u64(seed);

    let mut sets: Vec<Vec<&String>> = Default::default();

    for i in cli.limited_depth..RANDOM_MAX_DEPTH {
        sets.extend(
            features
                .iter()
                .combinations(i + 1)
                .map(|x| x.into_iter().unique().collect_vec())
                .collect_vec(),
        );
    }

    let sets = sets
        .iter()
        .choose_multiple(&mut rng, count)
        .iter()
        .map(|x| x.iter().map(|x| x.to_string()).collect_vec())
        .collect_vec();

    Ok(sets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use roadster::testing::snapshot::TestCase;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(Cli::default(), &["a"], vec![], &[])]
    #[case(Cli::builder().limited_depth(5).build(), &["a", "b", "c"], vec![], &[])]
    #[case(Cli::builder().limited_depth(5).build(), &["a", "b", "c"], vec![vec!["a", "b"]], &[])]
    #[case(Cli::builder().limited_depth(5).build(), &["a", "b", "c"], vec![], &["c"])]
    #[case(Cli::builder().limited_depth(2).random_seed(1).random_count(1).build(), &["a", "b", "c", "d", "e", "f"], vec![], &[])]
    fn powerset_impl(
        _case: TestCase,
        #[case] cli: Cli,
        #[case] features: &[&str],
        #[case] groups: Vec<Vec<&str>>,
        #[case] skip: &[&str],
    ) {
        let features = features.iter().map(|s| s.to_string()).collect_vec();
        let groups = groups
            .iter()
            .map(|g| g.iter().map(|f| f.to_string()).collect())
            .collect_vec();
        let skip = skip.iter().map(|s| s.to_string()).collect_vec();

        let powerset = super::powerset_impl(&cli, features, groups, skip);

        assert_debug_snapshot!(powerset);
    }
}
