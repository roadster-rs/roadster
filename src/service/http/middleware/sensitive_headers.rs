use crate::app::context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::extract::FromRef;
use axum::http::{header, HeaderName};
use axum::Router;
use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use std::str::FromStr;

use crate::error::RoadsterResult;
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
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
    pub fn header_names(&self) -> RoadsterResult<Vec<HeaderName>> {
        let header_names = self
            .header_names
            .iter()
            .map(|header_name| HeaderName::from_str(header_name))
            .try_collect()?;
        Ok(header_names)
    }
}

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct SensitiveRequestHeadersConfig {
    #[serde(flatten)]
    pub common: CommonSensitiveHeadersConfig,
}

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct SensitiveResponseHeadersConfig {
    #[serde(flatten)]
    pub common: CommonSensitiveHeadersConfig,
}

pub struct SensitiveRequestHeadersMiddleware;
impl<S> Middleware<S> for SensitiveRequestHeadersMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "sensitive-request-headers".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .sensitive_request_headers
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
            .sensitive_request_headers
            .common
            .priority
    }
    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let headers = AppContext::from_ref(state)
            .config()
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
impl<S> Middleware<S> for SensitiveResponseHeadersMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "sensitive-response-headers".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .sensitive_response_headers
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
            .sensitive_response_headers
            .common
            .priority
    }

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let headers = AppContext::from_ref(state)
            .config()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sensitive_request_headers_enabled(
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
            .sensitive_request_headers
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = SensitiveRequestHeadersMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, -10000)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sensitive_request_headers_priority(
        #[case] override_priority: Option<i32>,
        #[case] expected_priority: i32,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .sensitive_request_headers
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = SensitiveRequestHeadersMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sensitive_response_headers_enabled(
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
            .sensitive_response_headers
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = SensitiveResponseHeadersMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 10000)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn sensitive_response_headers_priority(
        #[case] override_priority: Option<i32>,
        #[case] expected_priority: i32,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .sensitive_response_headers
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = SensitiveResponseHeadersMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
