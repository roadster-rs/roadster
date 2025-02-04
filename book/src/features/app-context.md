# App context

The [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html) is the core container for
shared state in Roadster. It can be used as the Axum [Router](https://docs.rs/axum/latest/axum/struct.Router.html) state
directly, or you can define your own state type that can be used by both Roadster and Axum by
implementing [FromRef](https://docs.rs/axum-core/latest/axum_core/extract/trait.FromRef.html).

```rust,ignore
{{#include ../../examples/app-context/src/state.rs}}
```

## `Provide` and `ProvideRef` traits

The [Provide](https://docs.rs/roadster/latest/roadster/app/context/trait.Provide.html)
and [ProvideRef](https://docs.rs/roadster/latest/roadster/app/context/trait.ProvideRef.html) traits allow getting
an instance of `T` from the implementing
type. [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html) implements this for
various
types it contains. This allows
a method to specify the type it requires, then the caller of the method can determine how to provide the type. This is a
similar concept to dependency injection (DI) in frameworks like Java Spring, though this is far from a full DI system.

This is useful, for example, to allow mocking the DB connection in tests. Your DB operation method would declare a
parameter of type `ProvideRef<DataBaseConnection>`, then your application code would provide
the [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html) to the
method, and your tests could provide a
mocked [ProvideRef](https://docs.rs/roadster/latest/roadster/app/context/trait.ProvideRef.html) instance that returns a
mock DB connection. Note that mocking
the DB comes with its own set of trade-offs, for example, it may not exactly match the behavior of an actual DB that's
used in production. Consider testing against an actual DB instead of mocking, e.g., by using test containers.

Mocked implementations of the traits are provided if the `testing-mocks` feature is enabled.

```rust,ignore
{{#include ../../examples/app-context/src/provide.rs:5:}}
```

See also:

- [SeaORM Mock Interface](https://www.sea-ql.org/SeaORM/docs/write-test/mock/)
- [Test Containers](https://testcontainers.com/)
- [Roadster Testing docs](https://roadster.dev/features/testing.html/)

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

## See also

- [Axum State chapter](/features/services/http/state.html)

## Docs.rs links

- [AppContext](https://docs.rs/roadster/latest/roadster/app/context/struct.AppContext.html)
