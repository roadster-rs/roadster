# Axum Middleware

Roadster's HTTP service has full support for any Axum (or Tower) middleware, many of which are provided by default. It's
also possible to register middleware that Roadster doesn't provide by default, or even create and provide your own
custom middleware.

## Default middleware

For the most up to date list of middleware provided by Roadster,
see the [module's docs.rs page](https://docs.rs/roadster/latest/roadster/service/http/middleware/index.html)

All of the default middleware can be configured via the app's config files. All middleware have at least the following
config fields:

- `enable`: Whether the middleware is enabled. If not provided, the middleware enablement falls back to the value of the
  `service.http.middleware.default-enable` field.
- `priority`: The priority in which the middleware will run. Lower values (including negative numbers) run before higher
  values. The middlewares provided by Roadster have priorities between `-10,000` (runs first) and `10,000` (runs later)
  by default, though these values can be overridden via configs. If the order your middleware runs in doesn't matter,
  simply set to `0`.

## Custom middleware

Custom middleware can be provided by implementing the [
`Middleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/trait.Middleware.html) trait. As a
convenience, custom middleware can also be applied using the [
`AnyMiddleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/any/struct.AnyMiddleware.html)
utility. This is useful, for example, for middleware that can be built using Axum's [
`from_fn`](https://docs.rs/axum/latest/axum/middleware/fn.from_fn.html) method.

```rust,ignore
{{#include ../../../../examples/service/src/http/middleware.rs:14:}}
```

## Docs.rs links

- [middleware mod](https://docs.rs/roadster/latest/roadster/service/http/middleware/index.html)
- [`Middleware` trait](https://docs.rs/roadster/latest/roadster/service/http/middleware/trait.Middleware.html)
- [`AnyMiddleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/any/struct.AnyMiddleware.html)