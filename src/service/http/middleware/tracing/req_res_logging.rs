//! Middleware to log the request/response payloads. Logs at the debug level.

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::body::{Body, Bytes};
use axum::extract::{FromRef, Request, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{middleware, Router};
use http_body_util::BodyExt;
use mime::Mime;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::BTreeSet;
use std::str::FromStr;
use tracing::debug;
use validator::Validate;

pub use RequestResponseLoggingConfig as ReqResLoggingConfig;
pub use RequestResponseLoggingMiddleware as RequestLoggingMiddleware;

#[serde_as]
#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct RequestResponseLoggingConfig {
    /// The maximum length to log. Payloads longer than this length will be truncated. To log the
    /// entire payload without any truncating, set to a negative number.
    pub max_len: i32,

    /// The content types to log. If not provided, all content types will be logged unless
    /// otherwise specified via `content_types_req` or `content_types_res`.
    ///
    /// Note: this currently only supports exact matches, so using `text/*` will not match
    /// `text/plain`, for example.
    #[serde_as(as = "Option<BTreeSet<DisplayFromStr>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_types: Option<BTreeSet<Mime>>,

    /// The request payload content types to log. If not provided, will fall back to the
    /// values from `content_types`.
    ///
    /// Note: this currently only supports exact matches, so using `text/*` will not match
    /// `text/plain`, for example.
    #[serde_as(as = "Option<BTreeSet<DisplayFromStr>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_types_req: Option<BTreeSet<Mime>>,

    /// The response payload content types to log. If not provided, will fall back to the
    /// values from `content_types`.
    ///
    /// Note: this currently only supports exact matches, so using `text/*` will not match
    /// `text/plain`, for example.
    #[serde_as(as = "Option<BTreeSet<DisplayFromStr>>")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_types_res: Option<BTreeSet<Mime>>,
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

        let router = router.layer(middleware::from_fn_with_state(
            state.clone(),
            move |State(state): State<S>, request, next| {
                log_req_res_bodies(state, request, next, max_len)
            },
        ));

        Ok(router)
    }
}

const TRUNCATED_STR: &str = "[truncated according to the `service.http.middleware.request-response-logging.max-len` config]";
const CONTENT_TYPE_OMITTED_STR: &str = "[omitted according to the `service.http.middleware.request-response-logging.content_types*` configs]";

// https://github.com/tokio-rs/axum/blob/main/examples/consume-body-in-extractor-or-middleware/src/main.rs
async fn log_req_res_bodies<S>(
    state: S,
    request: Request,
    next: Next,
    max_len: i32,
) -> Result<impl IntoResponse, Response>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(&state);
    let config = &context
        .config()
        .service
        .http
        .custom
        .middleware
        .request_response_logging
        .custom;

    // Log the request body
    let is_req = true;
    let (parts, body) = request.into_parts();
    let content_type = get_content_type(parts.headers.get(CONTENT_TYPE))
        .ok()
        .flatten();
    let body = if should_log_content_type(config, content_type.as_ref(), is_req).unwrap_or_default()
    {
        let bytes = log_body(body, max_len, is_req).await?;
        Body::from(bytes)
    } else {
        let content_type = content_type
            .as_ref()
            .map(|content_type| content_type.as_ref())
            .unwrap_or_default();
        debug!(content_type, body = CONTENT_TYPE_OMITTED_STR, "request");
        body
    };
    let request = Request::from_parts(parts, body);

    // Handle the request
    let response = next.run(request).await;

    // Log the response body
    let is_req = false;
    let (parts, body) = response.into_parts();
    let content_type = get_content_type(parts.headers.get(CONTENT_TYPE))
        .ok()
        .flatten();
    let body = if should_log_content_type(config, content_type.as_ref(), is_req).unwrap_or_default()
    {
        let bytes = log_body(body, max_len, is_req).await?;
        Body::from(bytes)
    } else {
        let content_type = content_type
            .as_ref()
            .map(|content_type| content_type.as_ref())
            .unwrap_or_default();
        debug!(content_type, body = CONTENT_TYPE_OMITTED_STR, "response");
        body
    };
    let response = Response::from_parts(parts, body);

    // Return the response
    Ok(response)
}

fn get_content_type(content_type: Option<&HeaderValue>) -> RoadsterResult<Option<Mime>> {
    if let Some(content_type) = content_type {
        Ok(Some(Mime::from_str(content_type.to_str()?)?))
    } else {
        Ok(None)
    }
}

fn should_log_content_type(
    config: &RequestResponseLoggingConfig,
    content_type: Option<&Mime>,
    is_req: bool,
) -> RoadsterResult<bool> {
    let config = if is_req {
        (
            config.content_types.as_ref(),
            config.content_types_req.as_ref(),
        )
    } else {
        (
            config.content_types.as_ref(),
            config.content_types_res.as_ref(),
        )
    };
    // Todo: Is there a cleaner way to write this?
    match config {
        (Some(a), Some(b)) => {
            if let Some(content_type) = content_type {
                Ok(a.contains(content_type) || b.contains(content_type))
            } else {
                Ok(false)
            }
        }
        (None, Some(a)) => {
            if let Some(content_type) = content_type {
                Ok(a.contains(content_type))
            } else {
                Ok(false)
            }
        }
        (Some(a), None) => {
            if let Some(content_type) = content_type {
                Ok(a.contains(content_type))
            } else {
                Ok(false)
            }
        }
        (None, None) => Ok(true),
    }
}

async fn log_body(body: Body, max_len: i32, is_req: bool) -> Result<Bytes, Response> {
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

    if is_req {
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

    #[rstest]
    #[case(
        r#"
        max-len = -1
        "#,
        None,
        true,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types = []
        "#,
        None,
        true,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types = []
        "#,
        None,
        false,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types-req = []
        "#,
        None,
        true,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types-res = []
        "#,
        None,
        true,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types-req = []
        "#,
        None,
        false,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types-res = []
        "#,
        None,
        false,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types = ["text/plain"]
        "#,
        Some("text/plain"),
        true,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types = ["text/plain"]
        "#,
        Some("text/plain"),
        false,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types-req = ["text/plain"]
        "#,
        Some("text/plain"),
        true,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types-res = ["text/plain"]
        "#,
        Some("text/plain"),
        false,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types = ["application/json"]
        "#,
        Some("text/html"),
        true,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types = ["application/json"]
        "#,
        Some("text/html"),
        false,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types-req = ["application/json"]
        "#,
        Some("text/html"),
        true,
        false
    )]
    #[case(
        r#"
        max-len = -1
        content-types-req = ["application/json"]
        "#,
        Some("text/html"),
        false,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types-res = ["application/json"]
        "#,
        Some("text/html"),
        true,
        true
    )]
    #[case(
        r#"
        max-len = -1
        content-types-res = ["application/json"]
        "#,
        Some("text/html"),
        false,
        false
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn should_log_content_type(
        #[case] config: &str,
        #[case] content_type: Option<&str>,
        #[case] is_req: bool,
        #[case] expected: bool,
    ) {
        let config: RequestResponseLoggingConfig = toml::from_str(config).unwrap();
        let content_type = content_type.map(|s| Mime::from_str(s).unwrap());

        let should_log =
            super::should_log_content_type(&config, content_type.as_ref(), is_req).unwrap();

        assert_eq!(should_log, expected);
    }
}
