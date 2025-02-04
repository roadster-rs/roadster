# Services

An [AppService](https://docs.rs/roadster/latest/roadster/service/trait.AppService.html) is a long-running, persistent
task that's the primary way to add functionality to your app. Roadster provides some `AppService`s, such
as [HttpService](https://docs.rs/roadster/latest/roadster/service/http/index.html), [SidekiqWorkerService](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/service/index.html),
and the general [FunctionService](https://docs.rs/roadster/latest/roadster/service/function/service/index.html).

## Registering a service

In order to run a service in your app, it needs to be registered with the service registry.

```rust,ignore
{{#include ../../../examples/service/src/lib.rs:8:}}
```

## Docs.rs links

- [`service` mod](https://docs.rs/roadster/latest/roadster/service/index.html)
