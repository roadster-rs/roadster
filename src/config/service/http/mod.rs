#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::service::http::initializer::Initializer;
use crate::config::service::http::middleware::Middleware;
use crate::controller::http::build_path;
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

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[validate(schema(function = "validate_default_routes"))]
pub struct DefaultRoutes {
    #[serde(default = "default_true")]
    pub default_enable: bool,
    #[serde(default = "DefaultRouteConfig::default_ping")]
    pub ping: DefaultRouteConfig,
    #[serde(default = "DefaultRouteConfig::default_health")]
    pub health: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    #[serde(default = "DefaultRouteConfig::default_api_schema")]
    pub api_schema: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    #[serde(default = "DefaultRouteConfig::default_scalar")]
    pub scalar: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    #[serde(default = "DefaultRouteConfig::default_redoc")]
    pub redoc: DefaultRouteConfig,
}

impl Default for DefaultRoutes {
    fn default() -> Self {
        Self {
            default_enable: default_true(),
            ping: DefaultRouteConfig::default_ping(),
            health: DefaultRouteConfig::default_health(),
            #[cfg(feature = "open-api")]
            api_schema: DefaultRouteConfig::default_api_schema(),
            #[cfg(feature = "open-api")]
            scalar: DefaultRouteConfig::default_scalar(),
            #[cfg(feature = "open-api")]
            redoc: DefaultRouteConfig::default_redoc(),
        }
    }
}

fn validate_default_routes(default_routes: &DefaultRoutes) -> Result<(), ValidationError> {
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

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DefaultRouteConfig {
    pub enable: Option<bool>,
    pub route: String,
}

impl DefaultRouteConfig {
    fn new(route: &str) -> Self {
        Self {
            enable: None,
            route: build_path("", route),
        }
    }

    fn default_ping() -> Self {
        DefaultRouteConfig::new("_ping")
    }

    fn default_health() -> Self {
        DefaultRouteConfig::new("_health")
    }

    #[cfg(feature = "open-api")]
    fn default_api_schema() -> Self {
        DefaultRouteConfig::new("_docs/api.json")
    }

    #[cfg(feature = "open-api")]
    fn default_scalar() -> Self {
        DefaultRouteConfig::new("_docs")
    }

    #[cfg(feature = "open-api")]
    fn default_redoc() -> Self {
        DefaultRouteConfig::new("_docs/redoc")
    }

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
