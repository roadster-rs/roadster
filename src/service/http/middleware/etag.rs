use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::extract::{FromRef, Request};
use axum::http::header::ETAG;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{middleware, Router};
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct EtagConfig {}

pub struct EtagMiddleware;
impl<S> Middleware<S> for EtagMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "etag".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .etag
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
            .etag
            .common
            .priority
    }

    fn install(&self, router: Router, _state: &S) -> RoadsterResult<Router> {
        let router = router.layer(middleware::from_fn(etag_middleware));

        Ok(router)
    }
}

async fn etag_middleware(request: Request, next: Next) -> Response {
    let request_headers = request.headers();
    let request_etag = etag_value_from_headers(request_headers).map(|etag| etag.to_string());

    let response = next.run(request).await;

    if request_etag.is_none() {
        return response;
    }

    let response_headers = response.headers();
    let response_etag = etag_value_from_headers(response_headers);

    if let Some((request_etag, response_etag)) = request_etag.zip(response_etag) {
        if request_etag == response_etag {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    response
}

fn etag_value_from_headers(headers: &HeaderMap) -> Option<&str> {
    headers.get(ETAG).and_then(|etag| etag.to_str().ok())
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
    fn enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config.service.http.custom.middleware.etag.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = EtagMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, 0)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config.service.http.custom.middleware.etag.common.priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = EtagMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
