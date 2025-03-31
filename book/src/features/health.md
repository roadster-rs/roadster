# Health checks

Roadster allows registering [
`HealthCheck`](https://docs.rs/roadster/latest/roadster/health_check/trait.HealthCheck.html)s to ensure server
instances are functioning as expected. Roadster provides some default health checks that simply check that the server's
dependencies (e.g., DB and Redis) are accessible. All health checks -- both the defaults and any custom ones registered
for an app -- run on app startup, via the CLI, and in the API at `/api/_health`. The route of this API is configurable
via the `service.http.default-routes.health.route` config field.

Roadster also provides the `/api/_ping` API, which simply returns a successful HTTP status (200) and does no
other work.

## Custom `HealthCheck`

To provide a custom health check, implement the [
`HealthCheck`](https://docs.rs/roadster/latest/roadster/health_check/trait.HealthCheck.html) trait and register the
check when building the app. Note that if the check requires access to the app's state, it should be provided via a [
`Weak`](https://doc.rust-lang.org/std/sync/struct.Weak.html) reference to the state. This is because health checks
are stored in Roadster's [`AppContext`](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html),
which introduces a circular reference between the context and health checks. A weak reference to `AppContext` can be
retrieved via [
`AppContext#downgrade`](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html#method.downgrade).

### Implement `HealthCheck`

```rust,ignore
{{#include ../../examples/health-check/src/example_check.rs:7:}}
```

### Register custom `HealthCheck`

```rust,ignore
{{#include ../../examples/health-check/src/lib.rs:7:}}
```

## Docs.rs links

- [`HealthCheck` trait](https://docs.rs/roadster/latest/roadster/health_check/trait.HealthCheck.html)
- [`HealthCheck` config](https://docs.rs/roadster/latest/roadster/config/health_check/struct.HealthCheck.html)
