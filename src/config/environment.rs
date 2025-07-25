use crate::config::{ENV_VAR_PREFIX, ENV_VAR_SEPARATOR};
use crate::error::RoadsterResult;
#[cfg(feature = "cli")]
use clap::ValueEnum;
#[cfg(feature = "cli")]
use clap::builder::PossibleValue;
use const_format::concatcp;
use dotenvy::dotenv;
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

pub(crate) const ENVIRONMENT_ENV_VAR_NAME: &str = "ENVIRONMENT";

const ENV_VAR_WITH_PREFIX: &str =
    concatcp!(ENV_VAR_PREFIX, ENV_VAR_SEPARATOR, ENVIRONMENT_ENV_VAR_NAME);

impl Environment {
    // This can run before tracing is initialized, so we need to use `println` in order to
    // log from this method.
    #[allow(clippy::disallowed_macros)]
    pub fn new() -> RoadsterResult<Self> {
        dotenv().ok();

        // Get the stage, and validate it by parsing to the Environment enum
        let environment = env::var(ENV_VAR_WITH_PREFIX).map_err(|_| {
            crate::error::other::OtherError::Message(format!(
                "Env var `{ENV_VAR_WITH_PREFIX}` not defined."
            ))
        })?;
        let environment = Self::from_str_impl(&environment, true);
        println!("Using environment from `{ENV_VAR_WITH_PREFIX}` env var: {environment:?}");
        Ok(environment)
    }

    fn value_variants_impl<'a>() -> &'a [Self] {
        ENV_VARIANTS.get_or_init(|| {
            vec![
                Environment::Development,
                Environment::Test,
                Environment::Production,
                Environment::Custom("<custom>".to_string()),
            ]
        })
    }

    fn from_str_impl(input: &str, ignore_case: bool) -> Self {
        Self::value_variants_impl()
            .iter()
            .find(|variant| {
                let values = variant.to_possible_value_impl();
                if ignore_case {
                    values
                        .iter()
                        .any(|value| value.to_lowercase() == input.to_lowercase())
                } else {
                    values.iter().any(|value| value == input)
                }
            })
            .cloned()
            .unwrap_or_else(|| Environment::Custom(input.to_string()))
    }

    fn to_possible_value_impl(&self) -> Vec<String> {
        match self {
            Environment::Development => vec![DEVELOPMENT.to_string(), "dev".to_string()],
            Environment::Test => vec![TEST.to_string()],
            Environment::Production => vec![PRODUCTION.to_string(), "prod".to_string()],
            Environment::Custom(custom) => vec![custom.to_string()],
        }
    }
}

// We need to manually implement (vs. deriving) `ValueEnum` in order to support the
// `Environment::Custom` variant.
#[cfg(feature = "cli")]
impl ValueEnum for Environment {
    fn value_variants<'a>() -> &'a [Self] {
        Self::value_variants_impl()
    }

    fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
        Ok(Self::from_str_impl(input, ignore_case))
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        let values = self.to_possible_value_impl();
        values
            .first()
            .map(PossibleValue::new)
            .map(|possible_value| possible_value.aliases(&values[1..]))
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
        Ok(Self::from_str_impl(s, true))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::{assert_debug_snapshot, assert_json_snapshot, assert_toml_snapshot};
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
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
    #[case(DEVELOPMENT.to_string())]
    #[case("dev".to_string())]
    #[case(TEST.to_string())]
    #[case(PRODUCTION.to_string())]
    #[case("prod".to_string())]
    #[case("custom-environment".to_string())]
    #[case(DEVELOPMENT.to_uppercase())]
    #[case(TEST.to_uppercase())]
    #[case(PRODUCTION.to_uppercase())]
    #[case("custom-environment".to_uppercase())]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn environment_from_str(_case: TestCase, #[case] env: String) {
        let env = <Environment as FromStr>::from_str(&env).unwrap();
        assert_debug_snapshot!(env);
    }

    #[rstest]
    #[case(DEVELOPMENT.to_string())]
    #[case(DEVELOPMENT.to_uppercase())]
    #[cfg(feature = "cli")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn value_enum_from_str(_case: TestCase, #[case] env: String) {
        let env = <Environment as ValueEnum>::from_str(&env, false).unwrap();
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
