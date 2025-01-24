use async_trait::async_trait;
use axum_core::extract::FromRef;
use clap::Parser;
use serde_derive::{Deserialize, Serialize};
use strum_macros::{EnumString, IntoStaticStr};
use tracing::info;

use crate::api::cli::roadster::{RoadsterCli, RunRoadsterCommand};
use crate::app::context::AppContext;
use crate::app::App;
use crate::config::AppConfig;
use crate::error::RoadsterResult;

#[derive(Debug, Parser, Serialize)]
#[non_exhaustive]
pub struct PrintConfigArgs {
    /// Print the config with the specified format.
    #[clap(short, long, default_value = "debug")]
    pub format: Format,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Serialize, Deserialize, EnumString, IntoStaticStr, clap::ValueEnum,
)]
#[serde(rename_all = "kebab-case", tag = "type")]
#[strum(serialize_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    Debug,
    Json,
    JsonPretty,
    Toml,
    TomlPretty,
}

#[async_trait]
impl<A, S> RunRoadsterCommand<A, S> for PrintConfigArgs
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S>,
{
    async fn run(&self, _app: &A, _cli: &RoadsterCli, state: &S) -> RoadsterResult<bool> {
        let context = AppContext::from_ref(state);
        let serialized = serialize_config(&self.format, context.config())?;

        info!("\n{}", serialized);

        Ok(true)
    }
}

fn serialize_config(format: &Format, config: &AppConfig) -> RoadsterResult<String> {
    let serialized = match format {
        Format::Debug => {
            format!("{:?}", config)
        }
        Format::Json => serde_json::to_string(config)?,
        Format::JsonPretty => serde_json::to_string_pretty(config)?,
        Format::Toml => toml::to_string(config)?,
        Format::TomlPretty => toml::to_string_pretty(config)?,
    };
    Ok(serialized)
}

#[cfg(all(
    test,
    feature = "http",
    feature = "open-api",
    feature = "sidekiq",
    feature = "db-sql",
    feature = "email-smtp",
    feature = "email-sendgrid",
    feature = "jwt",
    feature = "jwt-ietf",
    feature = "otel",
    feature = "grpc",
    feature = "testing",
    feature = "test-containers",
    feature = "testing-mocks",
    feature = "config-yml",
))]
mod tests {
    use crate::api::cli::roadster::print_config::Format;
    use crate::config::AppConfig;
    use crate::testing::snapshot::TestCase;
    use insta::assert_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn config() -> AppConfig {
        AppConfig::test(None).unwrap()
    }

    #[rstest]
    #[case(Format::Debug)]
    #[case(Format::Json)]
    #[case(Format::JsonPretty)]
    #[case(Format::Toml)]
    #[case(Format::TomlPretty)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialize_config(_case: TestCase, config: AppConfig, #[case] format: Format) {
        let serialized = super::serialize_config(&format, &config).unwrap();

        assert_snapshot!(serialized);
    }
}
