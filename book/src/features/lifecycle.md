# Lifecycle handlers

Roadster provides the [
`AppLifecycleHandler`](https://docs.rs/roadster/latest/roadster/lifecycle/trait.AppLifecycleHandler.html) trait to allow
apps to hook into various stages of their lifecycle to perform any custom setup or teardown logic. Roadster also
implements some of its built-in lifecycle logic using this trait; for example, DB migrations are performed by the [
`DbMigrationLifecycleHandler`](https://docs.rs/roadster/latest/roadster/lifecycle/db_migration/struct.DbMigrationLifecycleHandler.html).
Similar to the HTTP middleware and initializers, lifecycle handlers are run in priority order, and the priority of each
handler can be customized via the app's config files.

## Lifecycle hooks

`AppLifecycleHandler`s have various hooks to allow managing resources at a particular point in the process of
the app's lifecycle. They are listed below in the order in which they are run:

1. `before_health_checks`: Runs right before the
   app's [health checks](https://docs.rs/roadster/latest/roadster/health_check/trait.HealthCheck.html) are run during
   startup.
2. `before_services`: Runs right before the
   app's [services](https://docs.rs/roadster/latest/roadster/service/trait.AppService.html) are started.
3. `on_shutdown`: Runs when the app is shutting down after all the
   app's [services](https://docs.rs/roadster/latest/roadster/service/trait.AppService.html) have been stopped.

## Default lifecycle handlers

For the most up to date list of lifecycle handlers provided by Roadster, see
the [module's docs.rs page](https://docs.rs/roadster/latest/roadster/lifecycle/index.html)

All of the default lifecycle handlers can be configured via the app's config files. All lifecycle handlers have at least
the following config fields:

- `enable`: Whether the lifecycle handlers is enabled. If not provided, the lifecycle handlers enablement falls back to
  the value of the `lifecycle-handler.default-enable` field.
- `priority`: The priority in which the lifecycle handlers will run. Lower values (including negative numbers) run
  before higher values. The lifecycle handlers provided by Roadster have priorities between `-10,000` (runs first) and
  `10,000` (runs later) by default, though these values can be overridden via configs. If the order your lifecycle
  handlers runs in doesn't matter, simply set to `0`.

## Custom middleware

### Implement `AppLifecycleHandler`

Custom middleware can be provided by implementing the [
`AppLifecycleHandler`](https://docs.rs/roadster/latest/roadster/lifecycle/trait.AppLifecycleHandler.html) trait.

```rust,ignore
{{#include ../../examples/lifecycle/src/example_lifecycle_handler.rs:7:}}
```

### Register the handler

In order to run for the handler to run, it needs to be registered with the app.

```rust,ignore
{{#include ../../examples/lifecycle/src/lib.rs:7:}}
```

## Docs.rs links

- [`AppLifecycleHandler` trait](https://docs.rs/roadster/latest/roadster/lifecycle/trait.AppLifecycleHandler.html)

