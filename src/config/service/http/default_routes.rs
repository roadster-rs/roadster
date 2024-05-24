use crate::app_context::AppContext;
use crate::util::serde_util;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;
use validator::ValidationError;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[validate(schema(function = "validate_default_routes"))]
pub struct DefaultRoutes {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[serde(deserialize_with = "deserialize_ping", default = "default_ping")]
    pub ping: DefaultRouteConfig,

    #[serde(deserialize_with = "deserialize_health", default = "default_health")]
    pub health: DefaultRouteConfig,

    #[cfg(feature = "open-api")]
    #[serde(
        deserialize_with = "deserialize_api_schema",
        default = "default_api_schema"
    )]
    pub api_schema: DefaultRouteConfig,

    #[cfg(feature = "open-api")]
    #[serde(deserialize_with = "deserialize_scalar", default = "default_scalar")]
    pub scalar: DefaultRouteConfig,

    #[cfg(feature = "open-api")]
    #[serde(deserialize_with = "deserialize_redoc", default = "default_redoc")]
    pub redoc: DefaultRouteConfig,
}

impl Default for DefaultRoutes {
    fn default() -> Self {
        Self {
            default_enable: default_true(),
            ping: default_ping(),
            health: default_health(),
            #[cfg(feature = "open-api")]
            api_schema: default_api_schema(),
            #[cfg(feature = "open-api")]
            scalar: default_scalar(),
            #[cfg(feature = "open-api")]
            redoc: default_redoc(),
        }
    }
}

fn validate_default_routes(
    // This parameter isn't used for some feature flag combinations
    #[allow(unused)] default_routes: &DefaultRoutes,
) -> Result<(), ValidationError> {
    #[cfg(feature = "open-api")]
    {
        let default_enable = default_routes.default_enable;
        let api_schema_enabled = default_routes.api_schema.enable.unwrap_or(default_enable);
        let scalar_enabled = default_routes.scalar.enable.unwrap_or(default_enable);
        let redoc_enabled = default_routes.redoc.enable.unwrap_or(default_enable);

        if scalar_enabled && !api_schema_enabled {
            return Err(ValidationError::new(
                "The Open API schema route must be enabled in order to use the Scalar docs route.",
            ));
        }
        if redoc_enabled && !api_schema_enabled {
            return Err(ValidationError::new(
                "The Open API schema route must be enabled in order to use the Redoc docs route.",
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DefaultRouteConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    pub route: String,
}

impl DefaultRouteConfig {
    pub fn enabled<S>(&self, context: &AppContext<S>) -> bool {
        self.enable.unwrap_or(
            context
                .config()
                .service
                .http
                .custom
                .default_routes
                .default_enable,
        )
    }
}

// This fun boilerplate allows the user to
// 1. Partially override a config without needing to provide all of the required values for the config
// 2. Prevent a type's `Default` implementation from being used and overriding the default we
//    actually want. For example, we provide a default for the `route` fields, and we want that
//    value to be used if the user doesn't provide one, not the type's default (`""` in this case).
//
// See: https://users.rust-lang.org/t/serde-default-value-for-struct-field-depending-on-parent/73452/2
//
// This is mainly needed because all of the default routes share a struct for their common configs,
// so we can't simply set a default on the field directly with a serde annotation.
// An alternative implementation could be to have different structs for each default route's common
// config instead of sharing a struct type. However, that would still require a lot of boilerplate.

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
struct PartialDefaultRouteConfig {
    pub enable: Option<bool>,
    pub route: Option<String>,
}

fn deserialize_ping<'de, D>(deserializer: D) -> Result<DefaultRouteConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config("_ping".to_string()))
}

fn default_ping() -> DefaultRouteConfig {
    deserialize_ping(serde_util::empty_json_object()).unwrap()
}

fn deserialize_health<'de, D>(deserializer: D) -> Result<DefaultRouteConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config("_health".to_string()))
}

fn default_health() -> DefaultRouteConfig {
    deserialize_health(serde_util::empty_json_object()).unwrap()
}

#[cfg(feature = "open-api")]
fn deserialize_api_schema<'de, D>(deserializer: D) -> Result<DefaultRouteConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer)
        .map(map_empty_config("_docs/api.json".to_string()))
}

#[cfg(feature = "open-api")]
fn default_api_schema() -> DefaultRouteConfig {
    deserialize_api_schema(serde_util::empty_json_object()).unwrap()
}

#[cfg(feature = "open-api")]
fn deserialize_scalar<'de, D>(deserializer: D) -> Result<DefaultRouteConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config("_docs".to_string()))
}

#[cfg(feature = "open-api")]
fn default_scalar() -> DefaultRouteConfig {
    deserialize_scalar(serde_util::empty_json_object()).unwrap()
}

#[cfg(feature = "open-api")]
fn deserialize_redoc<'de, D>(deserializer: D) -> Result<DefaultRouteConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde::Deserialize::deserialize(deserializer).map(map_empty_config("_docs/redoc".to_string()))
}

#[cfg(feature = "open-api")]
fn default_redoc() -> DefaultRouteConfig {
    deserialize_redoc(serde_util::empty_json_object()).unwrap()
}

fn map_empty_config(
    default_route: String,
) -> impl FnOnce(PartialDefaultRouteConfig) -> DefaultRouteConfig {
    move |PartialDefaultRouteConfig { enable, route }| DefaultRouteConfig {
        enable,
        route: route.unwrap_or(default_route),
    }
}

#[cfg(test)]
mod tests {
    use crate::config::service::http::*;
    use rstest::rstest;

    #[rstest]
    #[case(false, false)]
    #[case(true, false)]
    #[cfg(not(feature = "open-api"))]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn validate_default_routes(#[case] default_enable: bool, #[case] validation_error: bool) {
        // Arrange
        #[allow(clippy::field_reassign_with_default)]
        let config = {
            let mut config = DefaultRoutes::default();
            config.default_enable = default_enable;
            config
        };

        // Act
        let result = config.validate();

        // Assert
        assert_eq!(result.is_err(), validation_error);
    }

    #[rstest]
    #[case(false, None, None, None, false)]
    #[case(true, None, None, None, false)]
    #[case(false, None, Some(true), None, true)]
    #[case(false, None, None, Some(true), true)]
    #[case(false, Some(true), Some(true), None, false)]
    #[case(false, Some(true), None, Some(true), false)]
    #[cfg(feature = "open-api")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn validate_default_routes(
        #[case] default_enable: bool,
        #[case] api_schema_enabled: Option<bool>,
        #[case] scalar_enabled: Option<bool>,
        #[case] redoc_enabled: Option<bool>,
        #[case] validation_error: bool,
    ) {
        // Arrange
        #[allow(clippy::field_reassign_with_default)]
        let config = {
            let mut config = DefaultRoutes::default();
            config.default_enable = default_enable;
            config.api_schema.enable = api_schema_enabled;
            config.scalar.enable = scalar_enabled;
            config.redoc.enable = redoc_enabled;
            config
        };

        // Act
        let result = config.validate();

        // Assert
        assert_eq!(result.is_err(), validation_error);
    }
}

// To simplify testing, these are only run when all of the config fields are available
#[cfg(all(test, feature = "open-api"))]
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
    #[case("")]
    #[case(
        r#"
        default-enable = false
        [ping]
        enable = true
        [health]
        enable = true
        [api-schema]
        enable = true
        [scalar]
        enable = true
        [redoc]
        enable = true
        "#
    )]
    #[case(
        r#"
        default-enable = false
        [ping]
        enable = false
        [health]
        enable = false
        [api-schema]
        enable = false
        [scalar]
        enable = false
        [redoc]
        enable = false
        "#
    )]
    #[case(
        r#"
        default-enable = false
        [ping]
        route = "a"
        [health]
        route = "b"
        [api-schema]
        route = "c"
        [scalar]
        route = "d"
        [redoc]
        route = "e"
        "#
    )]
    #[case(
        r#"
        [ping]
        enable = true
        route = "a"
        [health]
        enable = true
        route = "b"
        [api-schema]
        enable = true
        route = "c"
        [scalar]
        enable = true
        route = "d"
        [redoc]
        enable = true
        route = "e"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn auth(_case: TestCase, #[case] config: &str) {
        let default_routes: DefaultRoutes = toml::from_str(config).unwrap();

        assert_toml_snapshot!(default_routes);
    }
}
