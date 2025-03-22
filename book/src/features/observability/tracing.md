# Tracing with Tokio's [tracing](https://crates.io/crates/tracing) crate

Roadster provides support for tracing as defined by the
[`init_tracing`](https://docs.rs/roadster/latest/roadster/tracing/fn.init_tracing.html) method, which is used in the
default implementation of [
`App#init_tracing`](https://docs.rs/roadster/latest/roadster/app/trait.App.html#method.init_tracing). Some of the
initialization logic can be configured via the app's config files. The app can also provide its own custom tracing
initialization logic.

If the `otel` feature is enabled, this method will also initialize some OpenTelemetry resources. See
the [OpenTelemetry chapter](otel.md) for more details.

## Configuration

```toml
{{ #include ../../../examples/tracing/config/development.toml}}
```

See the [Tracing config struct](https://docs.rs/roadster/latest/roadster/config/tracing/struct.Tracing.html) for the
full list of available fields.

## Custom initialization logic

If the app has custom requirements for tracing / metrics, custom logic can be provided. Note that if a custom
implementation is provided, none of the default tracing setup
from [`init_tracing`](https://docs.rs/roadster/latest/roadster/tracing/fn.init_tracing.html) will be applied.

```rust,ignore
{{#include ../../../examples/tracing/src/lib.rs}}
```

## Provided trace events/logic for HTTP requests

When the `http` feature is enabled, Roadster provides some additional tracing features via Axum/Tower middleware.

### HTTP Request ID

Roadster can generate an ID for each HTTP request. This is done with the [
`SetRequestIdMiddleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/request_id/struct.SetRequestIdMiddleware.html),
which is a wrapper around `tower-http`'s [
`SetRequestIdLayer`](https://docs.rs/tower-http/latest/tower_http/request_id/struct.SetRequestId.html). This layer will
generate a new request ID (as a UUID) for each HTTP request if one wasn't provided in the request.

Additionally, Roadster allows your app to propagate the request ID to any service it calls. This is done with the [
`PropagateRequestIdMiddleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/request_id/struct.PropagateRequestIdMiddleware.html),
which is a wrapper around `tower-http`'s [
`PropagateRequestIdLayer`](https://docs.rs/tower-http/latest/tower_http/request_id/struct.PropagateRequestIdLayer.html).

The request ID is fetched and propagated via an HTTP request header, which is configurable via the `header-name` field
of each middleware's config.

```toml
{{ #include ../../../examples/tracing/config/development/request_id_middleware.toml}}
```

Each of these middlewares can also be disabled is desired.

### HTTP request/response events

In addition to generating an ID for each request, Roadster can also create a tracing span for each request. This is done
with the [
`TracingMiddleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/tracing/struct.TracingMiddleware.html),
which is a wrapper around `tower-http`'s [
`TraceLayer`](https://docs.rs/tower-http/latest/tower_http/trace/struct.TraceLayer.html), with some custom logic for how
to create spans and emit events on request and response.

The span includes the request ID as an attribute. The ID is retrieved from the header name configured for the
`SetRequestIdMiddleware`. Below is a sample trace (as logs):

```text
2025-03-20T08:22:01.257662Z  INFO roadster::service::http::middleware::tracing: started processing request, version: HTTP/1.1, url.path: /api/_health, request_headers: {"host": "localhost:3000", "user-agent": "<redacted>", "accept": "application/json, text/plain, */*", "accept-language": "en-US,en;q=0.5", "accept-encoding": "gzip, deflate, br, zstd", "connection": "keep-alive", "referer": "http://localhost:3000/api/_docs", "sec-fetch-dest": "empty", "sec-fetch-mode": "cors", "sec-fetch-site": "same-origin", "dnt": "1", "sec-gpc": "1", "priority": "u=0", "request-id": "9727c5c8-a982-42ef-8d7e-c3e388d378ae"}
    at src/service/http/middleware/tracing/mod.rs:141
    in roadster::service::http::middleware::tracing::http_request with http.request.method: GET, http.route: /api/_health, request_id: 9727c5c8-a982-42ef-8d7e-c3e388d378ae

2025-03-20T08:22:01.269580Z  INFO tower_http::trace::on_response: finished processing request, latency: 12 ms, status: 200, response_headers: {"content-type": "application/json", "request-id": "9727c5c8-a982-42ef-8d7e-c3e388d378ae", "vary": "origin, access-control-request-method, access-control-request-headers"}
    at /Users/<redacted>/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tower-http-0.6.2/src/trace/on_response.rs:114
    in roadster::service::http::middleware::tracing::http_request with http.request.method: GET, http.route: /api/_health, request_id: 9727c5c8-a982-42ef-8d7e-c3e388d378ae, http.response.status_code: 200
```

### HTTP request/response payload logging for debugging

In non-prod environments, it can be very helpful to be able to inspect request and response payloads. Roadster allows
this via the custom [
`RequestResponseLoggingMiddleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/tracing/req_res_logging/struct.RequestResponseLoggingMiddleware.html).
Note that this middleware does not work for requests/responses that are long-running streams.

Technically it's possible to enable this middleware in prod via app configs; however,
***this is strongly discouraged*** as this can leak sensitive data into your trace events and/or logs. If logging
request/response payloads in prod is desired, the payload should be encrypted before it's logged. Alternatively,
something like the [`secrecy`](https://docs.rs/secrecy/latest/secrecy/) crate could be used for sensitive struct
fields, then the rest of the struct can be safely logged from your API handler.

## Docs.rs links

- [`init_tracing` method](https://docs.rs/roadster/latest/roadster/tracing/fn.init_tracing.html)
- [`Tracing` config struct](https://docs.rs/roadster/latest/roadster/config/tracing/struct.Tracing.html)
- [`request_id` middleware
  `mod`](https://docs.rs/roadster/latest/roadster/service/http/middleware/request_id/index.html)