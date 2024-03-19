use std::env;
use std::str::FromStr;

use anyhow::anyhow;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};

#[derive(Debug, Clone, Serialize, Deserialize, EnumString, IntoStaticStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    pub fn new() -> anyhow::Result<Self> {
        // Get the stage, and validate it by parsing to the Environment enum
        // Todo: allow specifying the environment via a CLI arg as well
        let environment = env::var("ENVIRONMENT").expect("Env var `ENVIRONMENT` not defined.");
        let environment = Environment::from_str(&environment).map_err(|err| {
            anyhow!("Unable to parse `ENVIRONMENT` env var with value `{environment}`: {err}")
        })?;
        Ok(environment)
    }
}
