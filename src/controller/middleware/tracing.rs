use crate::app_context::AppContext;
use crate::controller::middleware::request_id::REQUEST_ID_HEADER_NAME;
use crate::controller::middleware::Middleware;
use axum::extract::MatchedPath;
use axum::http::{Request, Response};
use axum::Router;
use opentelemetry_semantic_conventions::trace::{
    HTTP_REQUEST_METHOD, HTTP_RESPONSE_STATUS_CODE, HTTP_ROUTE, URL_PATH,
};
use std::time::Duration;
use tower_http::trace::{DefaultOnResponse, MakeSpan, OnRequest, OnResponse, TraceLayer};
use tracing::{event, field, info_span, Level, Span, Value};

pub struct TracingMiddleware;
impl Middleware for TracingMiddleware {
    fn name(&self) -> String {
        "tracing".to_string()
    }

    fn install(&self, router: Router, _context: &AppContext) -> anyhow::Result<Router> {
        let router = router.layer(
            TraceLayer::new_for_http()
                .make_span_with(CustomMakeSpan)
                .on_request(CustomOnRequest)
                .on_response(CustomOnResponse::new()),
        );

        Ok(router)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CustomMakeSpan;

impl<B> MakeSpan<B> for CustomMakeSpan {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let path = get_path(request);
        let request_id = get_request_id(request);
        info_span!("request",
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

fn get_request_id<B>(request: &Request<B>) -> Option<&str> {
    request
        .headers()
        // TODO: Is there a built-in way to get this header?
        .get(REQUEST_ID_HEADER_NAME)
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
