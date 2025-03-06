# Initializers

[`Initializer`](https://docs.rs/roadster/latest/roadster/service/http/initializer/trait.Initializer.html)s are similar
to [
`Middleware`](https://docs.rs/roadster/latest/roadster/service/http/middleware/trait.Middleware.html) -- they both
allow configuring the Axum [`Router`](https://docs.rs/axum/latest/axum/struct.Router.html) for your app's HTTP service.
However, `Initializer`s provide more precise control over when it is applied to the `Router`. This is useful to apply
middleware that requires a fully set up `Router` in order to work as expected, e.g. Tower's [
`NormalizePathLayer`](https://docs.rs/tower-http/0.6.2/tower_http/normalize_path/struct.NormalizePathLayer.html). It's
also useful in order to initialize [`Extension`](https://docs.rs/axum/latest/axum/struct.Extension.html)s that you
want to attach to the `Router` -- this is most useful for using external utility crates that expect state to be in an
`Extension`, most state you need in your own application code should probably be in a [custom state struct](./state.md).

## Initializer hooks

`Initializer`s have various hooks to allow modifying the `Router` at a particular point in the process of building it.
They are listed below in the order in which they are run:

1. `after_router`: Runs after all of the routes have been added to the `Router`
2. `before_middleware`: Runs before any [`Middleware`](./middleware.md) is added to the `Router`.
3. `after_middleware`: Runs after all [`Middleware`](./middleware.md) has been added to the `Router`.
4. `before_serve`: Runs right before the HTTP service starts.

## Default initializers

Currently, the only `Initializer` provided by Roadster is the [`NormalizePathInitializer`]. This initializer applies
Tower's [
`NormalizePathLayer`](https://docs.rs/tower-http/0.6.2/tower_http/normalize_path/struct.NormalizePathLayer.html) after
all other `Router` setup has completed, which is required in order for it to properly normalize paths (e.g., treat paths
with and without trailing slashes as the same path).

If more `Initializer`s are added in the future, they can be found in
the [module's docs.rs page](https://docs.rs/roadster/latest/roadster/service/http/middleware/index.html).

All of the default initializers can be configured via the app's config files. All initializers have at least the
following config fields:

- `enable`: Whether the initializer is enabled. If not provided, the initializer enablement falls back to the value of
  the `service.http.initializer.default-enable` field.
- `priority`: The priority in which the initializer will run. Lower values (including negative numbers) run before
  higher
  values. The middlewares provided by Roadster have priorities between `-10,000` (runs first) and `10,000` (runs later)
  by default, though these values can be overridden via configs. If the order your initializer runs in relative to other
  initializers doesn't matter, simply set to `0`.

## Custom initializers

Custom initializers can be provided by implementing the [
`Initializer`](https://docs.rs/roadster/latest/roadster/service/http/initializer/trait.Initializer.html) trait. As a
convenience, custom initializers can also be applied using the [
`AnyInitializer`](https://docs.rs/roadster/latest/roadster/service/http/initializer/any/struct.AnyInitializer.html)
utility. This is useful to run an initializer without adding a full struct + trait implementation.

```rust,ignore
{{#include ../../../../examples/service/src/http/initializer.rs:11:}}
```

## Docs.rs links

- [`Initializer` trait](https://docs.rs/roadster/latest/roadster/service/http/initializer/trait.Initializer.html)
- [initializer mod](https://docs.rs/roadster/latest/roadster/service/http/initializer/index.html)
- [`AnyInitializer`](https://docs.rs/roadster/latest/roadster/service/http/initializer/any/struct.AnyInitializer.html)

