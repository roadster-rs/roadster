use crate::config::{ENV_VAR_PREFIX, ENV_VAR_SEPARATOR};
use crate::error::RoadsterResult;
use anyhow::anyhow;
use clap::builder::PossibleValue;
#[cfg(feature = "cli")]
use clap::ValueEnum;
use const_format::concatcp;
use serde_derive::{Deserialize, Serialize};
use std::env;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::OnceLock;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Environment {
    Development,
    Test,
    Production,
    #[serde(untagged)]
    Custom(String),
}

static ENV_VARIANTS: OnceLock<Vec<Environment>> = OnceLock::new();

const DEVELOPMENT: &str = "development";
const TEST: &str = "test";
const PRODUCTION: &str = "production";

// We need to manually implement (vs. deriving) `ValueEnum` in order to support the
// `Environment::Custom` variant.
#[cfg(feature = "cli")]
impl ValueEnum for Environment {
    fn value_variants<'a>() -> &'a [Self] {
        ENV_VARIANTS.get_or_init(|| {
            vec![
                Environment::Development,
                Environment::Test,
                Environment::Production,
                Environment::Custom("<custom>".to_string()),
            ]
        })
    }

    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        let env = Self::value_variants()
            .iter()
            .find(|v| {
                v.to_possible_value()
                    .map(|v| v.matches(input, ignore_case))
                    .unwrap_or_default()
            })
            .cloned()
            .unwrap_or_else(|| Environment::Custom(input.to_string()))
            .clone();

        Ok(env)
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        match self {
            Environment::Development => Some(PossibleValue::new(DEVELOPMENT).alias("dev")),
            Environment::Test => Some(PossibleValue::new(TEST)),
            Environment::Production => Some(PossibleValue::new(PRODUCTION).alias("prod")),
            Environment::Custom(custom) => Some(
                PossibleValue::new(custom).help("Any other value will be captured as a String."),
            ),
        }
    }
}

// We need to manually implement `Display` (vs. deriving `IntoStaticStr` from `strum`) in order to
// support the `Environment::Custom` variant.
impl Display for Environment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => {
                write!(f, "{DEVELOPMENT}")
            }
            Environment::Test => {
                write!(f, "{TEST}")
            }
            Environment::Production => {
                write!(f, "{PRODUCTION}")
            }
            Environment::Custom(custom) => {
                write!(f, "{custom}")
            }
        }
    }
}

// We need to manually implement `FromStr` (vs. deriving `EnumString` from `strum`) in order to
// support the `Environment::Custom` variant.
impl FromStr for Environment {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let env = <Environment as ValueEnum>::from_str(s, true)?;
        Ok(env)
    }
}

/// Note: A future release may remove this implementation because it's not possible to convert
/// `Environment::Custom` to a static str. It's kept for now because it was implemented before
/// we added the `Environment::Custom` variant.
// todo: remove this implementation in a semver breaking version bump
impl From<Environment> for &'static str {
    fn from(value: Environment) -> Self {
        (&value).into()
    }
}

/// Note: A future release may remove this implementation because it's not possible to convert
/// `Environment::Custom` to a static str. It's kept for now because it was implemented before
/// we added the `Environment::Custom` variant.
// todo: remove this implementation in a semver breaking version bump
impl From<&Environment> for &'static str {
    fn from(value: &Environment) -> Self {
        match value {
            Environment::Development => DEVELOPMENT,
            Environment::Test => TEST,
            Environment::Production => PRODUCTION,
            Environment::Custom(_) => {
                unimplemented!("It's not possible to convert `Environment::Custom` to a static str. Use ToString/Display instead.")
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::{assert_debug_snapshot, assert_json_snapshot, assert_toml_snapshot};
    use rstest::{fixture, rstest};

    #[fixture]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(Environment::Development)]
    #[case(Environment::Test)]
    #[case(Environment::Production)]
    #[case(Environment::Custom("custom-environment".to_string()))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn environment_to_string(_case: TestCase, #[case] env: Environment) {
        let env = env.to_string();
        assert_debug_snapshot!(env);
    }

    #[rstest]
    #[case(Environment::Development, false)]
    #[case(Environment::Test, false)]
    #[case(Environment::Production, false)]
    #[case(Environment::Custom("custom-environment".to_string()), true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn environment_to_static_str(
        _case: TestCase,
        #[case] env: Environment,
        #[case] expect_error: bool,
    ) {
        let env = std::panic::catch_unwind(|| {
            let env: &str = env.into();
            env
        });
        assert_eq!(env.is_err(), expect_error);
    }

    #[rstest]
    #[case(DEVELOPMENT)]
    #[case("dev")]
    #[case(TEST)]
    #[case(PRODUCTION)]
    #[case("prod")]
    #[case("custom-environment")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn environment_from_str(_case: TestCase, #[case] env: &str) {
        let env = <Environment as FromStr>::from_str(env).unwrap();
        assert_debug_snapshot!(env);
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Wrapper {
        env: Environment,
    }

    #[rstest]
    #[case(Environment::Development)]
    #[case(Environment::Test)]
    #[case(Environment::Production)]
    #[case(Environment::Custom("custom-environment".to_string()))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn environment_serialize_json(_case: TestCase, #[case] env: Environment) {
        let env = Wrapper { env };
        assert_json_snapshot!(env);
    }

    #[rstest]
    #[case(Environment::Development)]
    #[case(Environment::Test)]
    #[case(Environment::Production)]
    #[case(Environment::Custom("custom-environment".to_string()))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn environment_serialize_toml(_case: TestCase, #[case] env: Environment) {
        let env = Wrapper { env };
        assert_toml_snapshot!(env);
    }
}
