#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::http::HeaderName;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

pub const REQUEST_ID_HEADER_NAME: &str = "request-id";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct CommonRequestIdConfig {
    pub header_name: String,
}

impl Default for CommonRequestIdConfig {
    fn default() -> Self {
        Self {
            header_name: REQUEST_ID_HEADER_NAME.to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct SetRequestIdConfig {
    #[serde(flatten)]
    pub common: CommonRequestIdConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct PropagateRequestIdConfig {
    #[serde(flatten)]
    pub common: CommonRequestIdConfig,
}

pub struct SetRequestIdMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for SetRequestIdMiddleware {
    fn name(&self) -> String {
        "set-request-id".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .set_request_id
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext<S>) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .set_request_id
            .common
            .priority
    }

    fn install(&self, router: Router, context: &AppContext<S>) -> anyhow::Result<Router> {
        let header_name = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .set_request_id
            .custom
            .common
            .header_name;

        let router = router.layer(SetRequestIdLayer::new(
            HeaderName::from_str(header_name)?,
            MakeRequestUuid,
        ));

        Ok(router)
    }
}

pub struct PropagateRequestIdMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for PropagateRequestIdMiddleware {
    fn name(&self) -> String {
        "propagate-request-id".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .propagate_request_id
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext<S>) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .propagate_request_id
            .common
            .priority
    }

    fn install(&self, router: Router, context: &AppContext<S>) -> anyhow::Result<Router> {
        let header_name = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .propagate_request_id
            .custom
            .common
            .header_name;

        let router = router.layer(PropagateRequestIdLayer::new(HeaderName::from_str(
            header_name,
        )?));

        Ok(router)
    }
}
