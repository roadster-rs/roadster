# Axum State

Axum allows providing a "[state](https://docs.rs/axum/latest/axum/extract/struct.State.html)" struct to
the [Router](https://docs.rs/axum/latest/axum/struct.Router.html). Roadster provides its state (DB connection pool, etc)
in the [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html) struct, which can
be used either as the Axum state directly. Or, if non-Roadster state is needed for some resource not provided by
Roadster, a custom struct can be used as long as it
implements [FromRef](https://docs.rs/axum/latest/axum/extract/trait.FromRef.html) so Roadster can get its `AppContext`
state from Axum.

## `FromRef` for custom state

`FromRef` can either be derived

```rust,ignore
{{#include ../../../../examples/app-context/src/state.rs:4:}}
```

or implemented manually

```rust,ignore
{{#include ../../../../examples/app-context/src/state_manual_from_ref.rs:4:}}
```

## Providing state

The app state needs to be provided to the `HttpService` when it's created. If
the [HttpServiceBuilder](https://docs.rs/roadster/latest/roadster/service/http/builder/struct.HttpServiceBuilder.html)
is used to register the service with
the [ServiceRegistry#register_builder](https://docs.rs/roadster/latest/roadster/service/registry/struct.ServiceRegistry.html#method.register_builder)
method, the state will be provided automatically when the `ServiceRegistry` builds the service.

```rust,ignore
{{#include ../../../../examples/service/src/lib.rs:8:}}
```

## See also

- [AppContext chapter](/features/app-context.html)

## Docs.rs links

- [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html)
- [Axum state](https://docs.rs/axum/latest/axum/extract/struct.State.html)
- [FromRef](https://docs.rs/axum/latest/axum/extract/trait.FromRef.html)
