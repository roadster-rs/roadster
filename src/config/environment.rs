use std::env;
use std::str::FromStr;

use anyhow::anyhow;
use const_format::concatcp;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};

use crate::config::app_config::{ENV_VAR_PREFIX, ENV_VAR_SEPARATOR};

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, IntoStaticStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
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
    pub fn new() -> anyhow::Result<Self> {
        // Get the stage, and validate it by parsing to the Environment enum
        let environment = env::var(ENV_VAR_WITH_PREFIX)
            .map_err(|_| anyhow!("Env var `{ENV_VAR_WITH_PREFIX}` not defined."))?;
        let environment = Environment::from_str(&environment).map_err(|err| {
            anyhow!(
                "Unable to parse `{ENV_VAR_WITH_PREFIX}` env var with value `{environment}`: {err}"
            )
        })?;
        println!("Using environment from `{ENV_VAR_WITH_PREFIX}` env var: {environment:?}");
        Ok(environment)
    }
}
