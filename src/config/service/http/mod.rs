#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::service::http::initializer::Initializer;
use crate::config::service::http::middleware::Middleware;
use crate::util::serde_util::default_true;
use serde_derive::{Deserialize, Serialize};
use validator::{Validate, ValidationError};

pub mod initializer;
pub mod middleware;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HttpServiceConfig {
    #[serde(flatten)]
    #[validate(nested)]
    pub address: Address,
    #[serde(default)]
    #[validate(nested)]
    pub middleware: Middleware,
    #[serde(default)]
    #[validate(nested)]
    pub initializer: Initializer,
    #[serde(default)]
    #[validate(nested)]
    pub default_routes: DefaultRoutes,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Address {
    pub host: String,
    pub port: u32,
}

impl Address {
    pub fn url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[validate(schema(function = "validate_default_routes"))]
pub struct DefaultRoutes {
    #[serde(default = "default_true")]
    pub default_enable: bool,
    #[serde(default)]
    pub ping: DefaultRouteConfig,
    #[serde(default)]
    pub health: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    #[serde(default)]
    pub api_schema: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    #[serde(default)]
    pub scalar: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    #[serde(default)]
    pub redoc: DefaultRouteConfig,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DefaultRouteConfig {
    pub enable: Option<bool>,
    pub route: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
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
