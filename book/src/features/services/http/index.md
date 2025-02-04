# HTTP Service with [Axum](https://crates.io/crates/axum)

The [HttpService](https://docs.rs/roadster/latest/roadster/service/http/service/struct.HttpService.html) provides
support for serving an HTTP API using [axum](https://docs.rs/axum/latest/axum/). The `HttpService` automatically applies
all the configured middleware and initializers automatically, so all that's needed in most cases to serve a production
ready API service is to define your routes, provide them to the `HttpService`, and register the `HttpService` with
the [ServiceRegistry](https://docs.rs/roadster/latest/roadster/service/registry/struct.ServiceRegistry.html).

```rust,ignore
{{#include ../../../../examples/service/src/http/mod.rs:14:}}
```

<details>
<summary>example_b module</summary>

```rust,ignore
{{#include ../../../../examples/service/src/http/example_b.rs:10:}}
```

</details>

<details>
<summary>example_c module</summary>

```rust,ignore
{{#include ../../../../examples/service/src/http/example_c.rs:10:}}
```

</details>

## OpenAPI Schema

If the `open-api` feature is enabled, the service also supports generating an OpenAPI schema. The OpenAPI schema can be
accessed in various ways.

### Via HTTP API

It's served by default at `/<base>/_docs/api.json`

```shell
# First, run your app
cargo run

# In a separate shell or browser, navigate to the API, e.g.
curl localhost:3000/api/_docs/api.json
```

### Via CLI command

It can be generated via a CLI command

```shell
cargo run -- roadster open-api -o $HOME/open-api.json
```

### Via the `HttpService` directly

It can also be generated programmatically using the `HttpService` directly.

```rust,ignore
{{#include ../../../../examples/service/src/http/open_api.rs:7:}}
```

## Docs.rs links

- [http service](https://docs.rs/roadster/latest/roadster/service/http/index.html)
