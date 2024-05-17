#[mockall_double::double]
use crate::app_context::AppContext;
use crate::config::service::http::initializer::Initializer;
use crate::config::service::http::middleware::Middleware;
use crate::controller::http::build_path;
use serde_derive::{Deserialize, Serialize};

pub mod initializer;
pub mod middleware;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HttpServiceConfig {
    #[serde(flatten)]
    pub address: Address,
    #[serde(default)]
    pub middleware: Middleware,
    #[serde(default)]
    pub initializer: Initializer,
    #[serde(default)]
    pub default_routes: DefaultRoutes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DefaultRoutes {
    pub default_enable: bool,
    pub ping: DefaultRouteConfig,
    pub health: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    pub api_schema: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    pub scalar: DefaultRouteConfig,
    #[cfg(feature = "open-api")]
    pub redoc: DefaultRouteConfig,
}

impl Default for DefaultRoutes {
    fn default() -> Self {
        Self {
            default_enable: true,
            ping: DefaultRouteConfig::new("_ping"),
            health: DefaultRouteConfig::new("_health"),
            #[cfg(feature = "open-api")]
            api_schema: DefaultRouteConfig::new("_docs/api.json"),
            #[cfg(feature = "open-api")]
            scalar: DefaultRouteConfig::new("_docs"),
            #[cfg(feature = "open-api")]
            redoc: DefaultRouteConfig::new("_docs/redoc"),
        }
    }
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
