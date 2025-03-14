# OpenAPI with [Aide](https://docs.rs/aide/0.14.1/aide/)

If the `open-api` feature is enabled, an OpenAPI schema can be built for the app's Axum API by registering API routes
using [Aide](https://docs.rs/aide/0.14.1/aide/). The schema will then be generated and served at the
`/api/_docs/api.json` route by default, and is also accessible via CLI commands or the [
`HttpService#open_api_schema`](https://docs.rs/roadster/latest/roadster/service/http/service/struct.HttpService.html#method.open_api_schema)
method.

## Register routes

OpenAPI routes are registered with Aide's [`ApiRouter`](https://docs.rs/aide/0.14.1/aide/axum/struct.ApiRouter.html),
which has a similar API to Axum's [`Router`](https://docs.rs/axum/latest/axum/struct.Router.html).

```rust,ignore
{{#include ../../examples/open-api/src/http/mod.rs:14:}}
```

## Get schema via API route

By default, the generated schema will be served at `/api/_docs/api.json`. This route can be configured via the
`service.http.default-routes.api-schema.route` config field.

```shell
# First, run your app
cargo run

# In a separate shell or browser, navigate to the API, e.g.
curl localhost:3000/api/_docs/api.json
```

## Get schema via CLI

The schema can also be generated via a CLI command

```shell
cargo run -- roadster open-api -o $HOME/open-api.json
```

## Get schema from the `HttpService`

The schema can also be generated programmatically using the `HttpService` directly.

```rust,ignore
{{#include ../../examples/service/src/http/open_api.rs:7:}}
```

## Docs.rs links

- [Aide](https://docs.rs/aide/0.14.1/aide/)
- [`HttpService`](https://docs.rs/roadster/latest/roadster/service/http/service/struct.HttpService.html)
- [`HttpServiceBuilder`](https://docs.rs/roadster/latest/roadster/service/http/builder/struct.HttpServiceBuilder.html)
