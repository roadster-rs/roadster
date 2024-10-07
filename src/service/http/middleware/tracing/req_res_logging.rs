//! Middleware to log the request/response payloads. Logs at the debug level.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::body::{Body, Bytes};
use axum::extract::{FromRef, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{middleware, Router};
use http_body_util::BodyExt;
use serde_derive::{Deserialize, Serialize};
use tracing::debug;
use validator::Validate;

pub use RequestResponseLoggingConfig as ReqResLoggingConfig;
pub use RequestResponseLoggingMiddleware as RequestLoggingMiddleware;

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct RequestResponseLoggingConfig {
    /// The maximum length to log. Payloads longer than this length will be truncated. To log the
    /// entire payload without any truncating, set to a negative number.
    pub max_len: i32,
}

pub struct RequestResponseLoggingMiddleware;
impl<S> Middleware<S> for RequestResponseLoggingMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "request-response-logging".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .request_response_logging
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
            .request_response_logging
            .common
            .priority
    }

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let max_len = AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .request_response_logging
            .custom
            .max_len;

        let router = router.layer(middleware::from_fn(move |request, next| {
            log_req_res_bodies(request, next, max_len)
        }));

        Ok(router)
    }
}

// https://github.com/tokio-rs/axum/blob/main/examples/consume-body-in-extractor-or-middleware/src/main.rs
async fn log_req_res_bodies(
    request: Request,
    next: Next,
    max_len: i32,
) -> Result<impl IntoResponse, Response> {
    // Log the request body
    let (parts, body) = request.into_parts();
    let bytes = log_body(body, max_len, true).await?;
    let request = Request::from_parts(parts, Body::from(bytes));

    // Handle the request
    let response = next.run(request).await;

    // Log the response body
    let (parts, body) = response.into_parts();
    let bytes = log_body(body, max_len, false).await?;
    let response = Response::from_parts(parts, Body::from(bytes));

    // Return the response
    Ok(response)
}

const TRUNCATED_STR: &str = "[truncated according to the `service.http.middleware.request-response-logging.max-len` config]";

async fn log_body(body: Body, max_len: i32, req: bool) -> Result<Bytes, Response> {
    // This only works if the body is not a long-running stream
    let bytes = body
        .collect()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
        .to_bytes();

    let body = if max_len == 0 {
        TRUNCATED_STR.to_string()
    } else if max_len < 0 || bytes.len() <= max_len as usize {
        format!("{bytes:?}")
    } else {
        assert!(max_len > 0);
        let slice = bytes.slice(0..(max_len as usize));
        format!("{slice:?}{TRUNCATED_STR}")
    };

    if req {
        debug!(body, "request");
    } else {
        debug!(body, "response");
    }

    Ok(bytes)
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
        config
            .service
            .http
            .custom
            .middleware
            .request_response_logging
            .common
            .enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = RequestResponseLoggingMiddleware;

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
            config
                .service
                .http
                .custom
                .middleware
                .request_response_logging
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = RequestResponseLoggingMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
