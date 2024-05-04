use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
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

impl<S> Middleware<S> for SensitiveRequestHeadersMiddleware {
    fn name(&self) -> String {
        "sensitive-request-headers".to_string()
    }

    fn enabled(&self, context: &AppContext, _state: &S) -> bool {
        context
            .config
            .service
            .http
            .custom
            .middleware
            .sensitive_request_headers
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext, _state: &S) -> i32 {
        context
            .config
            .service
            .http
            .custom
            .middleware
            .sensitive_request_headers
            .common
            .priority
    }
    fn install(&self, router: Router, context: &AppContext, _state: &S) -> anyhow::Result<Router> {
        let headers = context
            .config
            .service
            .http
            .custom
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

impl<S> Middleware<S> for SensitiveResponseHeadersMiddleware {
    fn name(&self) -> String {
        "sensitive-response-headers".to_string()
    }

    fn enabled(&self, context: &AppContext, _state: &S) -> bool {
        context
            .config
            .service
            .http
            .custom
            .middleware
            .sensitive_response_headers
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext, _state: &S) -> i32 {
        context
            .config
            .service
            .http
            .custom
            .middleware
            .sensitive_response_headers
            .common
            .priority
    }
    fn install(&self, router: Router, context: &AppContext, _state: &S) -> anyhow::Result<Router> {
        let headers = context
            .config
            .service
            .http
            .custom
            .middleware
            .sensitive_response_headers
            .custom
            .common
            .header_names()?;

        let router = router.layer(SetSensitiveResponseHeadersLayer::new(headers));

        Ok(router)
    }
}
