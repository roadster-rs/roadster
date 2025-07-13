use crate::app::context::AppContext;
use crate::util::serde::default_true;
use axum_core::extract::FromRef;
use serde_derive::{Deserialize, Serialize};
use validator::Validate;
use validator::ValidationError;

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[validate(schema(function = "validate_default_routes"))]
#[non_exhaustive]
pub struct DefaultRoutes {
    #[serde(default = "default_true")]
    pub default_enable: bool,

    #[validate(nested)]
    pub ping: DefaultRouteConfig,

    #[validate(nested)]
    pub health: DefaultRouteConfig,

    #[cfg(feature = "open-api")]
    #[validate(nested)]
    pub api_schema: DefaultRouteConfig,

    #[cfg(feature = "open-api")]
    #[validate(nested)]
    pub scalar: DefaultRouteConfig,

    #[cfg(feature = "open-api")]
    #[validate(nested)]
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

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct DefaultRouteConfig {
    #[serde(default)]
    pub enable: Option<bool>,
    pub route: String,
}

impl DefaultRouteConfig {
    pub fn enabled<S>(&self, state: &S) -> bool
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        self.enable.unwrap_or(
            AppContext::from_ref(state)
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
    use crate::config::AppConfig;
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
            let mut config = AppConfig::test(None)
                .unwrap()
                .service
                .http
                .custom
                .default_routes;
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
            let mut config = AppConfig::test(None)
                .unwrap()
                .service
                .http
                .custom
                .default_routes;
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
