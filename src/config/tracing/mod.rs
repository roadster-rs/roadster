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

    /// The interval (in milliseconds) at which OTEL metrics are exported.
    #[cfg(feature = "otel")]
    #[serde_as(as = "Option<serde_with::DurationMilliSeconds>")]
    pub metrics_export_interval: Option<std::time::Duration>,

    /// Configuration for OTLP exporters.
    #[validate(nested)]
    #[serde(default)]
    #[cfg(feature = "otel")]
    pub otlp: Option<Otlp>,
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

/// Configuration for OTLP exporters.
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[cfg(feature = "otel")]
pub struct Otlp {
    /// The endpoint to use for OTLP exporters if no trace/metric endpoint is provided.
    #[serde(default)]
    endpoint: Option<OtlpProtocol>,

    /// The endpoint to use for exporting traces via OTLP. If not provided, will use `endpoint`.
    #[serde(default)]
    trace_endpoint: Option<OtlpProtocol>,

    /// The endpoint to use for exporting metrics via OTLP. If not provided, will use `endpoint`.
    #[serde(default)]
    metric_endpoint: Option<OtlpProtocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "protocol")]
#[non_exhaustive]
#[cfg(feature = "otel")]
pub enum OtlpProtocol {
    Http(OtlpEndpoint),
    #[cfg(feature = "otel-grpc")]
    Grpc(OtlpEndpoint),
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
#[cfg(feature = "otel")]
pub struct OtlpEndpoint {
    pub url: Url,
}

#[cfg(feature = "otel")]
impl Otlp {
    pub fn trace_endpoint(&self) -> Option<&OtlpProtocol> {
        self.trace_endpoint.as_ref().or(self.endpoint.as_ref())
    }

    pub fn metric_endpoint(&self) -> Option<&OtlpProtocol> {
        self.metric_endpoint.as_ref().or(self.endpoint.as_ref())
    }
}

// To simplify testing, these are only run when all of the config fields are available
#[cfg(all(test, feature = "otel", feature = "otel-grpc"))]
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
        metrics-export-interval = 60000
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        [otlp.endpoint]
        protocol = "http"
        url = "https://example.com:1234"
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        [otlp.endpoint]
        protocol = "grpc"
        url = "https://example.com:1234"
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        [otlp.trace-endpoint]
        protocol = "http"
        url = "https://example.com:1234"
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        [otlp.trace-endpoint]
        protocol = "grpc"
        url = "https://example.com:1234"
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        [otlp.metric-endpoint]
        protocol = "http"
        url = "https://example.com:1234"
        "#
    )]
    #[case(
        r#"
        level = "debug"
        format = "none"
        [otlp.metric-endpoint]
        protocol = "grpc"
        url = "https://example.com:1234"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn tracing(_case: TestCase, #[case] config: &str) {
        let tracing: Tracing = toml::from_str(config).unwrap();

        assert_toml_snapshot!(tracing);
    }
}
