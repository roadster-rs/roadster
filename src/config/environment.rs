use std::env;
use std::str::FromStr;

use anyhow::anyhow;
#[cfg(feature = "cli")]
use clap::ValueEnum;
use const_format::concatcp;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};

use crate::config::app_config::{ENV_VAR_PREFIX, ENV_VAR_SEPARATOR};
use crate::error::RoadsterResult;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum Environment {
    Development,
    Test,
    Production,
}

pub(crate) const ENVIRONMENT_ENV_VAR_NAME: &str = "ENVIRONMENT";

const ENV_VAR_WITH_PREFIX: &str =
    concatcp!(ENV_VAR_PREFIX, ENV_VAR_SEPARATOR, ENVIRONMENT_ENV_VAR_NAME);
// This env var is used for backwards compatibility and may be removed in the next
// semver breaking release (0.4+)
const ENV_VAR_WITH_PREFIX_OLD: &str = concatcp!(ENV_VAR_PREFIX, ".", ENVIRONMENT_ENV_VAR_NAME);

impl Environment {
    // This runs before tracing is initialized, so we need to use `println` in order to
    // log from this method.
    #[allow(clippy::disallowed_macros)]
    pub fn new() -> RoadsterResult<Self> {
        // Get the stage, and validate it by parsing to the Environment enum
        let environment = if let Ok(value) = env::var(ENV_VAR_WITH_PREFIX) {
            println!("Using environment from `{ENV_VAR_WITH_PREFIX}` env var: {value:?}");
            value
        } else if let Ok(value) = env::var(ENV_VAR_WITH_PREFIX_OLD) {
            // This env var is used for backwards compatibility and may be removed in the next
            // semver breaking release (0.4+)
            println!("Using environment from `{ENV_VAR_WITH_PREFIX_OLD}` env var: {value:?}");
            value
        } else {
            Err(anyhow!("Neither `{ENV_VAR_WITH_PREFIX}` nor `{ENV_VAR_WITH_PREFIX_OLD}` env vars are defined."))?;
            unreachable!()
        };
        let environment = <Environment as FromStr>::from_str(&environment).map_err(|err| {
            anyhow!("Unable to parse environment from env var value `{environment}`: {err}")
        })?;
        println!("Parsed environment from env var: {environment:?}");
        Ok(environment)
    }
}
