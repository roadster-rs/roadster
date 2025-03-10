#[cfg(feature = "otel")]
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use strum_macros::{EnumString, IntoStaticStr};
#[cfg(feature = "otel")]
use url::Url;
use validator::Validate;

pub fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[serde_as]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Tracing {
    pub level: String,

    /// The format to use when printing traces to logs.
    pub format: Format,

    /// The name of the service to use for the OpenTelemetry `service.name` field. If not provided,
    /// will use the [`App::name`][crate::config::App] config value, translated to `snake_case`.
    #[cfg(feature = "otel")]
    pub service_name: Option<String>,

    /// Propagate traces across service boundaries. Mostly useful in microservice architectures.
    #[serde(default = "default_true")]
    #[cfg(feature = "otel")]
    pub trace_propagation: bool,

    /// URI of the OTLP exporter where traces/metrics/logs will be sent.
    #[cfg(feature = "otel")]
    pub otlp_endpoint: Option<Url>,

    /// The interval (in milliseconds) at which OTEL metrics are exported.
    #[cfg(feature = "otel")]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub metrics_export_interval: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    None,
    Pretty,
    Compact,
    Json,
}

// To simplify testing, these are only run when all of the config fields are available
#[cfg(all(test, feature = "otel"))]
mod deserialize_tests {
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
        level = "debug"
        format = "compact"
        "#
    )]
    #[case(
        r#"
        level = "info"
        format = "json"
        service-name = "foo"
        "#
    )]
    #[case(
        r#"
        level = "error"
        format = "pretty"
        trace-propagation = false
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        otlp-endpoint = "https://example.com:1234"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn tracing(_case: TestCase, #[case] config: &str) {
        let tracing: Tracing = toml::from_str(config).unwrap();

        assert_toml_snapshot!(tracing);
    }
}
