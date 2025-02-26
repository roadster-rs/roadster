use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use anyhow::anyhow;
use axum::Router;
use axum_core::extract::FromRef;
use byte_unit::Byte;
use byte_unit::Unit::MB;
use byte_unit::rust_decimal::prelude::ToPrimitive;
use serde_derive::{Deserialize, Serialize};
use tower_http::limit::RequestBodyLimitLayer;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct SizeLimitConfig {
    pub limit: Byte,
}

impl Default for SizeLimitConfig {
    fn default() -> Self {
        Self {
            #[allow(clippy::expect_used)]
            limit: Byte::from_u64_with_unit(5, MB).expect("Unable to build Byte unit."),
        }
    }
}

pub struct RequestBodyLimitMiddleware;
impl<S> Middleware<S> for RequestBodyLimitMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "request-body-size-limit".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .size_limit
            .common
            .enabled(state)
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .size_limit
            .common
            .priority
    }

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let limit = &AppContext::from_ref(state)
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

        let limit = match limit {
            Some(limit) => limit,
            None => return Err(anyhow!("Unable to convert bytes from u64 to usize").into()),
        };

        let router = router.layer(RequestBodyLimitLayer::new(*limit));

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn size_limit_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config
            .service
            .http
            .custom
            .middleware
            .size_limit
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = RequestBodyLimitMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, -9970)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn size_limit_priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .size_limit
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = RequestBodyLimitMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
