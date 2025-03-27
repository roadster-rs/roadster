pub mod req_res_logging;

use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::Router;
use axum::extract::{FromRef, MatchedPath};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Request, Response};
use opentelemetry_semantic_conventions::trace::{
    HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, HTTP_ROUTE, NETWORK_PROTOCOL_VERSION,
};
use serde_derive::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use std::time::Duration;
use tower_http::trace::{MakeSpan, OnRequest, OnResponse, TraceLayer};
use tracing::{Span, Value, field, info, info_span};
use validator::Validate;

#[derive(Debug, Clone, Default, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
#[non_exhaustive]
pub struct TracingConfig {
    /// Allow all HTTP request headers to be added as trace attributes. Useful for development and
    /// test environments. Not recommended to be enabled in production.
    #[serde(default)]
    pub request_headers_allow_all: bool,
    /// Allow all HTTP response headers to be added as trace attributes. Useful for development and
    /// test environments. Not recommended to be enabled in production.
    #[serde(default)]
    pub response_headers_allow_all: bool,
    /// Allow-list of HTTP request headers to add as trace attributes.
    #[serde(default)]
    pub request_header_names: Vec<String>,
    /// Allow-list of HTTP response headers to add as trace attributes.
    #[serde(default)]
    pub response_header_names: Vec<String>,
}

pub struct TracingMiddleware;
impl<S> Middleware<S> for TracingMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "tracing".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .tracing
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
            .tracing
            .common
            .priority
    }

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let context = AppContext::from_ref(state);
        let middleware_config = &context.config().service.http.custom.middleware;
        let request_id_header_name = &middleware_config.set_request_id.custom.common.header_name;
        let tracing_config = &middleware_config.tracing.custom;

        let router = router.layer(
            TraceLayer::new_for_http()
                .make_span_with(CustomMakeSpan::new(request_id_header_name))
                .on_request(CustomOnRequest::new(tracing_config))
                .on_response(CustomOnResponse::new(tracing_config)),
        );

        Ok(router)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CustomMakeSpan {
    pub request_id_header_name: String,
}

impl CustomMakeSpan {
    pub fn new(request_id_header_name: &str) -> Self {
        Self {
            request_id_header_name: request_id_header_name.to_owned(),
        }
    }
}

impl<B> MakeSpan<B> for CustomMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let matched_path = get_matched_path(request);
        let request_id = get_request_id(&self.request_id_header_name, request);

        /*
        The OTEL semantic conventions allow the span name to be `{method} {target}`,
        e.g., `GET /some/http/route`. However, the tracing crate we use requires the span name
        to be static. So, we use `HTTP` instead, which is the fallback value specified by OTEL.
         */
        info_span!("HTTP",
            { HTTP_REQUEST_METHOD } = %request.method(),
            { HTTP_ROUTE } = optional_trace_field(matched_path),
            { NETWORK_PROTOCOL_VERSION } = ?request.version(),
            request_id = optional_trace_field(request_id),
            // Fields that aren't know at request time, but may be known at response time
            { HTTP_RESPONSE_STATUS_CODE } = field::Empty,
        )
    }
}

fn get_matched_path<B>(request: &Request<B>) -> Option<&str> {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str())
}

fn get_request_id<'a, B>(
    request_id_header_name: &'a str,
    request: &'a Request<B>,
) -> Option<&'a str> {
    request
        .headers()
        .get(request_id_header_name)
        .and_then(|v| v.to_str().ok())
}

fn optional_trace_field<T>(value: Option<T>) -> Box<dyn Value>
where
    T: ToString,
{
    value
        .map(|x| Box::new(field::display(x.to_string())) as Box<dyn Value>)
        .unwrap_or(Box::new(field::Empty))
}

#[derive(Debug, Clone)]
pub struct CustomOnRequest {
    /// Allow all HTTP request headers to be added as trace attributes. Useful for development and
    /// test environments. Not recommended to be enabled in production.
    pub allow_all_headers: bool,
    /// Allow-list of HTTP request headers to add as trace attributes.
    pub request_header_names: HashSet<HeaderName>,
}

impl CustomOnRequest {
    pub fn new(tracing_config: &TracingConfig) -> Self {
        let request_header_names = tracing_config
            .request_header_names
            .iter()
            .filter_map(|name| HeaderName::from_str(name).ok())
            .collect();
        Self {
            allow_all_headers: tracing_config.request_headers_allow_all,
            request_header_names,
        }
    }
}

impl<B> OnRequest<B> for CustomOnRequest {
    fn on_request(&mut self, request: &Request<B>, _: &Span) {
        let request_headers = allowed_headers(
            request.headers(),
            &self.request_header_names,
            self.allow_all_headers,
        );

        /*
        The OTEL semantic conventions allow for providing request headers via the
        `http.request.header.<key>` span key. However, this is difficult to support with
        the tracing crate we're using because it requires specifying all span keys up front with
        static keys. This means we would need to manually list out all of the possible header
        names vs dynamically adding span keys. Instead, we include the headers as an attribute
        on the "request started" event.

        See: <https://docs.rs/tracing/latest/tracing/#recording-fields>
        See: <https://opentelemetry.io/docs/specs/semconv/attributes-registry/http/>
         */
        info!(?request_headers, "started processing request");
    }
}

#[derive(Debug, Clone)]
pub struct CustomOnResponse {
    /// Allow all HTTP response headers to be added as trace attributes. Useful for development and
    /// test environments. Not recommended to be enabled in production.
    pub allow_all_headers: bool,
    /// Allow-list of HTTP response headers to add as trace attributes.
    pub response_header_names: HashSet<HeaderName>,
}

impl CustomOnResponse {
    pub fn new(tracing_config: &TracingConfig) -> CustomOnResponse {
        let response_header_names = tracing_config
            .response_header_names
            .iter()
            .filter_map(|name| HeaderName::from_str(name).ok())
            .collect();
        CustomOnResponse {
            allow_all_headers: tracing_config.response_headers_allow_all,
            response_header_names,
        }
    }
}

impl<B> OnResponse<B> for CustomOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        span.record(HTTP_RESPONSE_STATUS_CODE, response.status().as_u16());

        let response_headers = allowed_headers(
            response.headers(),
            &self.response_header_names,
            self.allow_all_headers,
        );

        /*
        The OTEL semantic conventions allow for providing response headers via the
        `http.response.header.<key>` span key. However, this is difficult to support with
        the tracing crate we're using because it requires specifying all span keys up front with
        static keys. This means we would need to manually list out all of the possible header
        names vs dynamically adding span keys. Instead, we include the headers as an attribute
        on the "response started" event.

        See: <https://docs.rs/tracing/latest/tracing/#recording-fields>
        See: <https://opentelemetry.io/docs/specs/semconv/attributes-registry/http/>
         */
        info!(
            ?response_headers,
            // The latency can easily be derived from the trace itself. However, the `DefaultOnResponse`
            // impl includes the latency, so we include it as well.
            latency = latency.as_millis(),
            "finished processing request"
        )
    }
}

fn allowed_headers<'a>(
    headers: &'a HeaderMap,
    allow_list_headers: &'a HashSet<HeaderName>,
    allow_all: bool,
) -> BTreeMap<&'a str, &'a HeaderValue> {
    headers
        .iter()
        .filter(|(key, _)| allow_all || allow_list_headers.contains(*key))
        .map(|(key, value)| (key.as_str(), value))
        .collect()
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
    fn tracing_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config.service.http.custom.middleware.tracing.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = TracingMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, -9980)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn tracing_priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config
                .service
                .http
                .custom
                .middleware
                .tracing
                .common
                .priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = TracingMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }
}
