use crate::config::environment::Environment;
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub(crate) fn default_config_per_env(
    environment: Environment,
) -> Option<config::File<FileSourceString, FileFormat>> {
    let config = match environment {
        Environment::Production => Some(include_str!("config/production.toml")),
        _ => None,
    };
    config.map(|c| config::File::from_str(c, FileFormat::Toml))
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Sendgrid {
    pub api_key: String,
    #[serde(default = "default_true")]
    pub sandbox: bool,
    #[serde(default = "default_true")]
    pub https_only: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::assert_toml_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(
        r#"
        api-key = "api-key"
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        http_only = false
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        http_only = true
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        sandbox = false
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let sendgrid: Sendgrid = toml::from_str(config).unwrap();

        assert_toml_snapshot!(sendgrid);
    }
}
