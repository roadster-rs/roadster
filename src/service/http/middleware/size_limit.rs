#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use anyhow::bail;
use axum::Router;
use byte_unit::rust_decimal::prelude::ToPrimitive;
use byte_unit::Byte;
use byte_unit::Unit::MB;
use serde_derive::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct SizeLimitConfig {
    pub limit: Byte,
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            limit: Byte::from_u64_with_unit(5, MB).unwrap(),
        }
    }
}

pub struct RequestBodyLimitMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for RequestBodyLimitMiddleware {
    fn name(&self) -> String {
        "request-body-size-limit".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .size_limit
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
            .size_limit
            .common
            .priority
    }

    fn install(&self, router: Router, context: &AppContext<S>) -> anyhow::Result<Router> {
        let limit = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .size_limit
            .custom
            .limit
            .as_u64()
            .to_usize();

        // Todo: is there a cleaner way to write this?
        let limit = match limit {
            Some(limit) => limit,
            None => bail!("Unable to convert bytes from u64 to usize"),
        };

        let router = router.layer(RequestBodyLimitLayer::new(*limit));

        Ok(router)
    }
}
