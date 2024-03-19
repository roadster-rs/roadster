use crate::app_context::AppContext;
use crate::controller::middleware::Middleware;
use axum::http::{header, HeaderName};
use axum::Router;
use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonSensitiveHeadersConfig {
    pub header_names: Vec<String>,
}

impl Default for CommonSensitiveHeadersConfig {
    fn default() -> Self {
        Self {
            header_names: vec![
                header::AUTHORIZATION.to_string(),
                header::PROXY_AUTHORIZATION.to_string(),
                header::COOKIE.to_string(),
                header::SET_COOKIE.to_string(),
            ],
        }
    }
}

impl CommonSensitiveHeadersConfig {
    pub fn header_names(&self) -> anyhow::Result<Vec<HeaderName>> {
        let header_names = self
            .header_names
            .iter()
            .map(|header_name| HeaderName::from_str(header_name))
            .try_collect()?;
        Ok(header_names)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct SensitiveRequestHeadersConfig {
    #[serde(flatten)]
    pub common: CommonSensitiveHeadersConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct SensitiveResponseHeadersConfig {
    #[serde(flatten)]
    pub common: CommonSensitiveHeadersConfig,
}

pub struct SensitiveRequestHeadersMiddleware;

impl Middleware for SensitiveRequestHeadersMiddleware {
    fn name(&self) -> String {
        "sensitive-request-headers".to_string()
    }

    fn enabled(&self, context: &AppContext) -> bool {
        context
            .config
            .middleware
            .sensitive_request_headers
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext) -> i32 {
        context
            .config
            .middleware
            .sensitive_request_headers
            .common
            .priority
    }
    fn install(&self, router: Router, context: &AppContext) -> anyhow::Result<Router> {
        let headers = context
            .config
            .middleware
            .sensitive_request_headers
            .custom
            .common
            .header_names()?;

        let router = router.layer(SetSensitiveRequestHeadersLayer::new(headers));

        Ok(router)
    }
}

pub struct SensitiveResponseHeadersMiddleware;

impl Middleware for SensitiveResponseHeadersMiddleware {
    fn name(&self) -> String {
        "sensitive-response-headers".to_string()
    }

    fn enabled(&self, context: &AppContext) -> bool {
        context
            .config
            .middleware
            .sensitive_response_headers
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext) -> i32 {
        context
            .config
            .middleware
            .sensitive_response_headers
            .common
            .priority
    }
    fn install(&self, router: Router, context: &AppContext) -> anyhow::Result<Router> {
        let headers = context
            .config
            .middleware
            .sensitive_response_headers
            .custom
            .common
            .header_names()?;

        let router = router.layer(SetSensitiveResponseHeadersLayer::new(headers));

        Ok(router)
    }
}
