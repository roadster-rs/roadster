#[cfg(feature = "otel")]
use crate::util::serde_util::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "otel")]
use url::Url;
use validator::Validate;

pub fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Tracing {
    pub level: String,

    /// The name of the service to use for the OpenTelemetry `service.name` field. If not provided,
    /// will use the [`App::name`][crate::config::app_config::App] config value, translated to `snake_case`.
    #[cfg(feature = "otel")]
    pub service_name: Option<String>,

    /// Propagate traces across service boundaries. Mostly useful in microservice architectures.
    #[serde(default = "default_true")]
    #[cfg(feature = "otel")]
    pub trace_propagation: bool,

    /// URI of the OTLP exporter where traces/metrics/logs will be sent.
    #[cfg(feature = "otel")]
    pub otlp_endpoint: Option<Url>,
}

// To simplify testing, these are only run when all of the config fields are available
#[cfg(all(test, feature = "otel"))]
mod deserialize_tests {
    use super::*;
    use crate::util::test_util::TestCase;
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
        level = "debug"
        "#
    )]
    #[case(
        r#"
        level = "info"
        service-name = "foo"
        "#
    )]
    #[case(
        r#"
        level = "error"
        trace-propagation = false
        "#
    )]
    #[case(
        r#"
        level = "debug"
        otlp-endpoint = "https://example.com:1234"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sidekiq(_case: TestCase, #[case] config: &str) {
        let tracing: Tracing = toml::from_str(config).unwrap();

        assert_toml_snapshot!(tracing);
    }
}
