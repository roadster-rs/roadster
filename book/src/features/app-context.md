# App context

The [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html) is the core container for
shared state in Roadster. It can be used as the Axum [Router](https://docs.rs/axum/latest/axum/struct.Router.html) state
directly, or you can define your own state type that can be used by both Roadster and Axum by
implementing [FromRef](https://docs.rs/axum-core/latest/axum_core/extract/trait.FromRef.html).

```rust,ignore
{{#include ../../examples/app-context/src/state.rs}}
```

## `Provide` and `ProvideRef`

🛠️ todo 🛠️

- https://docs.rs/roadster/latest/roadster/app/context/trait.Provide.html
- https://docs.rs/roadster/latest/roadster/app/context/trait.ProvideRef.html

## Weak reference

In some cases, it can be useful to have a weak reference to the `AppContext` state in order to prevent reference cycles
for things that are included in the `AppContext` but also need a reference to the `AppContext`. For example, the
`AppContext` keeps a reference to the `HealthCheck`s, and most `HealthCheck`s need to use the `AppContext`.

To get a weak reference to the `AppContext`'s state,
use [AppContext#downgrade](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html#method.downgrade)
to get a new instance
of [AppContextWeak](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContextWeak.html).

```rust,ignore
{{#include ../../examples/app-context/src/app.rs:12:}}
```

## Docs.rs links

- [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html)
