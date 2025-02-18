# Roadster vs. [Axum](https://crates.io/crates/axum)

Roadster actually uses Axum to provide an HTTP server, so anything you can do with plain Axum you can do with Roadster.
However, using Roadster has some benefits compared to configuring Axum yourself:

- Roadster registers a collection of common middleware with sensible default configurations. The configurations can also
  be customized easily via config files. See [Axum middleware](../features/services/http/middleware.md) for more
  information.
- Roadster creates an `AppContext` to use as the Axum State that contains all the dependency objects created by
  Roadster, such as the DB connection, app config, etc. This can also be extended using
  Axum's [`FromRef`](https://docs.rs/axum/latest/axum/extract/derive.FromRef.html) if you need to provide additional
  state
  to your Axum routes.
  See [Axum state](../features/services/http/state.md) for more
  information.
- Roadster supports registering API routes using [Aide](https://crates.io/crates/aide) to enable auto-generating an
  OpenAPI schema and playground. See [OpenAPI with Aide](../features/open-api.md) for more information.
- Roadster auto-generates a unique request ID for each request, if one wasn't provided in the request
- Roadster configures the [Tracing](https://crates.io/crates/tracing) crate and enables instrumentation for requests.
  See [Tracing](../features/tracing/index.md) for more information.
