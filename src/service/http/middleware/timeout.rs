use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::Router;
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct TimeoutConfig {
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(10),
        }
    }
}

pub struct TimeoutMiddleware;
impl<S> Middleware<S> for TimeoutMiddleware {
    fn name(&self) -> String {
        "timeout".to_string()
    }

    fn enabled(&self, context: &AppContext, _state: &S) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .timeout
            .common
            .enabled(context)
    }

    fn priority(&self, context: &AppContext, _state: &S) -> i32 {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .timeout
            .common
            .priority
    }

    fn install(&self, router: Router, context: &AppContext, _state: &S) -> anyhow::Result<Router> {
        let timeout = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .timeout
            .custom
            .timeout;

        let router = router.layer(TimeoutLayer::new(*timeout));

        Ok(router)
    }
}
