use clap::Parser;
use strum_macros::{EnumString, IntoStaticStr};
use typed_builder::TypedBuilder;

#[derive(Debug, Default, Parser, TypedBuilder)]
#[command(version, about)]
#[non_exhaustive]
pub struct Cli {
    /// The output format
    #[clap(short, long, default_value = "debug")]
    #[builder(default)]
    pub format: Format,

    /// The size of each powerset group. If provided, the final powerset will be split into
    /// multiple groups of the given size. If not provided, the set will be provided in a single
    /// group.
    #[clap(short = 's', long)]
    #[builder(default, setter(strip_option))]
    pub group_size: Option<usize>,

    /// The maximum "depth" of the "limited" powerset. Each subset of the "limited" powerset
    /// will be at most this big.
    #[clap(short = 'd', long = "depth", default_value_t = 3)]
    pub limited_depth: usize,

    /// The number of subsets to return -- at random -- from the full powerset.
    #[builder(default, setter(strip_option))]
    #[clap(short = 'c', long)]
    pub random_count: Option<usize>,

    /// The value to use to seed the PRNG used to pick from the full powerset.
    #[builder(default, setter(strip_option))]
    #[clap(short = 'r', long)]
    pub random_seed: Option<u64>,
}

/// The output format
#[derive(Debug, Default, Clone, Eq, PartialEq, EnumString, IntoStaticStr, clap::ValueEnum)]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    #[default]
    Debug,
    Json,
    JsonPretty,
}
