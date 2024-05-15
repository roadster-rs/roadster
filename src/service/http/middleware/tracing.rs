#[mockall_double::double]
use crate::app_context::AppContext;
use crate::service::http::middleware::Middleware;
use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use axum::Router;
use opentelemetry_semantic_conventions::trace::{
    HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, HTTP_ROUTE, URL_PATH,
};
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;
use tower_http::trace::{DefaultOnResponse, MakeSpan, OnRequest, OnResponse, TraceLayer};
use tracing::{event, field, info_span, Level, Span, Value};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", default)]
pub struct TracingConfig {}

pub struct TracingMiddleware;
impl<S: Send + Sync + 'static> Middleware<S> for TracingMiddleware {
    fn name(&self) -> String {
        "tracing".to_string()
    }

    fn enabled(&self, context: &AppContext<S>) -> bool {
        context
            .config()
            .service
            .http
            .custom
            .middleware
            .tracing
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
            .tracing
            .common
            .priority
    }

    fn install(&self, router: Router, context: &AppContext<S>) -> anyhow::Result<Router> {
        let request_id_header_name = &context
            .config()
            .service
            .http
            .custom
            .middleware
            .set_request_id
            .custom
            .common
            .header_name;

        let router = router.layer(
            TraceLayer::new_for_http()
                .make_span_with(CustomMakeSpan::new(request_id_header_name.clone()))
                .on_request(CustomOnRequest)
                .on_response(CustomOnResponse::new()),
        );

        Ok(router)
    }
}

#[derive(Debug, Clone)]
pub struct CustomMakeSpan {
    pub request_id_header_name: String,
}

impl CustomMakeSpan {
    pub fn new(request_id_header_name: String) -> Self {
        Self {
            request_id_header_name,
        }
    }
}

impl<B> MakeSpan<B> for CustomMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let path = get_path(request);
        let request_id = get_request_id(&self.request_id_header_name, request);
        info_span!("http_request",
            { HTTP_REQUEST_METHOD } = %request.method(),
            { HTTP_ROUTE } = optional_trace_field(path),
            request_id = optional_trace_field(request_id),
            // Fields that aren't know at request time, but will (may?) be known by
            // response time
            { HTTP_RESPONSE_STATUS_CODE } = field::Empty,
        )
    }
}

fn get_path<B>(request: &Request<B>) -> Option<&str> {
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

#[derive(Debug, Copy, Clone)]
pub struct CustomOnRequest;

impl<B> OnRequest<B> for CustomOnRequest {
    fn on_request(&mut self, request: &Request<B>, _: &Span) {
        event!(
            Level::INFO,
            version = ?request.version(),
            { URL_PATH } = %request.uri(),
            request_headers = ?request.headers(),
            "started processing request",
        )
    }
}

#[derive(Debug, Clone)]
pub struct CustomOnResponse {
    default: DefaultOnResponse,
}

impl CustomOnResponse {
    pub fn new() -> CustomOnResponse {
        CustomOnResponse {
            default: DefaultOnResponse::new()
                .include_headers(true)
                // TODO: Configure the level via AppConfig?
                .level(Level::INFO),
        }
    }
}

impl Default for CustomOnResponse {
    fn default() -> Self {
        Self::new()
    }
}

impl<B> OnResponse<B> for CustomOnResponse {
    fn on_response(self, response: &Response<B>, latency: Duration, span: &Span) {
        span.record(HTTP_RESPONSE_STATUS_CODE, response.status().as_u16());
        self.default.on_response(response, latency, span);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_context::MockAppContext;
    use crate::config::app_config::AppConfig;
    use rstest::rstest;

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    fn tracing_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::empty(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config.service.http.custom.middleware.tracing.common.enable = enable;

        let mut context = MockAppContext::<()>::default();
        context.expect_config().return_const(config);

        let middleware = TracingMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }
}
