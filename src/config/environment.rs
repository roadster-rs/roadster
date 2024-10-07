use crate::config::{ENV_VAR_PREFIX, ENV_VAR_SEPARATOR};
use crate::error::RoadsterResult;
use anyhow::anyhow;
#[cfg(feature = "cli")]
use clap::ValueEnum;
use const_format::concatcp;
use serde_derive::{Deserialize, Serialize};
use std::env;
use std::str::FromStr;
use strum_macros::{EnumString, IntoStaticStr};

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

impl Environment {
    // This runs before tracing is initialized, so we need to use `println` in order to
    // log from this method.
    #[allow(clippy::disallowed_macros)]
    pub fn new() -> RoadsterResult<Self> {
        // Get the stage, and validate it by parsing to the Environment enum
        let environment = env::var(ENV_VAR_WITH_PREFIX)
            .map_err(|_| anyhow!("Env var `{ENV_VAR_WITH_PREFIX}` not defined."))?;
        let environment = <Environment as FromStr>::from_str(&environment).map_err(|err| {
            anyhow!(
                "Unable to parse `{ENV_VAR_WITH_PREFIX}` env var with value `{environment}`: {err}"
            )
        })?;
        println!("Using environment from `{ENV_VAR_WITH_PREFIX}` env var: {environment:?}");
        Ok(environment)
    }
}
