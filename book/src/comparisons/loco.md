# Roadster vs. [Loco](https://crates.io/crates/loco-rs)

Loco and Roadster serve similar purposes -- they both aim to reduce the amount of configuring, wiring, and other
boilerplate code required to build a backend or full-stack web app in Rust. There are some notable differences, however,
both in mission statement and the list of supported features. This section will give a summary of both the similarities
and differences between Loco and Roadster.

## Feature breakdown

Below is a detailed breakdown of the features built-in to Roadster and Loco. Note that because both frameworks are
based on Axum and Tokio, there's not a lot technically preventing either framework from implementing features they're
missing compared to the other. Features that Roadster would like to add in the near future are marked with '*'. Other
missing features are not planned but we'd be open to adding if there was interest in them.

*Last updated in July 2025.*

| Feature                                                                                                                         | Roadster       | Loco                                      |
|:--------------------------------------------------------------------------------------------------------------------------------|:---------------|:------------------------------------------|
| Separate `cargo` CLI to help with generating code and other tasks                                                               | ❌              | ✅                                         |
| Custom CLI commands                                                                                                             | ✅              | ✅                                         |
| HTTP APIs via Axum                                                                                                              | ✅              | ✅                                         |
| &ensp;↳ Default "ping" and "health" HTTP routes                                                                                 | ✅              | ✅                                         |
| &ensp;&ensp;↳ Default routes can be disabled via config                                                                         | ✅              | ❌                                         |
| &ensp;↳ Default middleware configured with sensible defaults                                                                    | ✅              | ✅                                         |
| &ensp;&ensp;↳ Middleware can be customized via config files                                                                     | ✅              | ✅                                         |
| &ensp;&ensp;↳ Middleware execution order can be customized via config files                                                     | ✅              | ❌                                         |
| OpenAPI support                                                                                                                 | ✅              | ✅                                         |
| &ensp;↳ built-in via [Aide](https://crates.io/crates/aide)                                                                      | ✅              | ❌                                         |
| &ensp;↳ 3rd party integration, e.g. [Utoipa](https://crates.io/crates/utoipa)                                                   | ✅              | ✅                                         |
| &ensp;↳ OpenAPI docs explorer http route provided by default                                                                    | ✅              | ❌                                         |
| GRPC API with [tonic](https://crates.io/crates/tonic)                                                                           | ✅              | ❌                                         |
| Channels (websockets and/or http long-polling)                                                                                  | ❌              | ✅                                         |
| Support for arbitrary long-running services                                                                                     | ✅              | ❌                                         |
| Health checks                                                                                                                   | ✅              | ✅                                         |
| &ensp;↳ Run in "health" API route                                                                                               | ✅              | ✅                                         |
| &ensp;↳ Run on app startup                                                                                                      | ✅              | ❌                                         |
| &ensp;↳ Run via CLI                                                                                                             | ✅              | ❌                                         |
| &ensp;↳ Consumer can provide custom checks                                                                                      | ✅              | ❌                                         |
| Custom app context / Axum state using Axum's [`FromRef`](https://docs.rs/axum-core/latest/axum_core/extract/trait.FromRef.html) | ✅              | ❌                                         |
| SQL DB                                                                                                                          | ✅              | ✅                                         |
| &ensp;↳ via Diesel                                                                                                              | ✅              | ❌                                         |
| &ensp;↳ via SeaORM                                                                                                              | ✅              | ✅                                         |
| &ensp;&ensp;↳ SeaORM migrations for common DB schemas                                                                           | ✅<br/>(in lib) | ✅<br/>(in starters)                       |
| Sample JWT Axum extractor                                                                                                       | ✅              | ✅                                         |
| &ensp;↳ Multiple JWT standards supported                                                                                        | ✅              | ❌                                         |
| Email                                                                                                                           | ✅              | ✅                                         |
| &ensp;↳ via SMTP                                                                                                                | ✅              | ✅                                         |
| &ensp;↳ via [Sendgrid's Mail Send API](https://www.twilio.com/docs/sendgrid/api-reference/mail-send/mail-send)                  | ✅              | ❌                                         |
| Storage abstraction                                                                                                             | ❌*             | ✅                                         |
| Cache abstraction                                                                                                               | ❌*             | ✅                                         |
| Background jobs                                                                                                                 | ✅              | ✅                                         |
| &ensp;↳ via Sidekiq                                                                                                             | ✅              | ✅                                         |
| &ensp;↳ via Postgres                                                                                                            | ✅              | ✅                                         |
| &ensp;↳ via in-process threading with Tokio                                                                                     | ❌              | ✅                                         |
| Periodic jobs                                                                                                                   | ✅              | ✅                                         |
| &ensp;↳ via Sidekiq                                                                                                             | ✅              | ❌                                         |
| &ensp;↳ via Postgres                                                                                                            | ✅              | ❌                                         |
| &ensp;↳ via custom scheduler                                                                                                    | ❌              | ✅                                         |
| Configuration via config files                                                                                                  | ✅              | ✅                                         |
| &ensp;↳ Toml                                                                                                                    | ✅              | ❌                                         |
| &ensp;↳ Yaml                                                                                                                    | ✅              | ✅                                         |
| Config files can be split into multiple files                                                                                   | ✅              | ❌                                         |
| Config values can be overridden via env vars                                                                                    | ✅              | ✅                                         |
| Config values can be fetch from async sources, such as secrets manager services                                                 | ✅              | ❌                                         |
| Tracing via the [tracing](https://crates.io/crates/tracing) crate                                                               | ✅              | ✅                                         |
| &ensp;↳ Built-in support for trace/metric exporting via OpenTelemetry                                                           | ✅              | ❌ <!--todo: double check-->               |
| Testing support                                                                                                                 | ✅              | ✅                                         |
| &ensp;↳ Test app bootstrapping                                                                                                  | ✅              | ✅                                         |
| &ensp;↳  [insta](https://crates.io/crates/insta) snapshot utilities                                                             | ✅              | ❌                                         |
| &ensp;↳ Data seeding and cleanup                                                                                                | ❌*             | ✅<br/>(⚠️ makes tests non-parallelizable) |
| Mock DB support for tests                                                                                                       | ✅              | ❌                                         |
| &ensp;↳ via Temporary Test DBs                                                                                                  | ✅              | ✅                                         |
| &ensp;↳ via SeaORM's [MockDatabase](https://www.sea-ql.org/SeaORM/docs/write-test/mock/)                                        | ✅              | ❌                                         |
| &ensp;↳ via [TestContainers](https://testcontainers.com/)                                                                       | ✅              | ❌                                         |
| Allows following any design pattern                                                                                             | ✅              | ❌<br/>(MVC only)                          |
| Lifecycle hooks                                                                                                                 | ✅              | ✅                                         |
| &ensp;↳ Customizable shutdown signal                                                                                            | ✅              | ❌                                         |
| HTML rendering                                                                                                                  | ✅              | ✅                                         |
| &ensp;↳ Built-in                                                                                                                | ❌              | ✅                                         |
| &ensp;↳ via 3rd party integration, e.g. [Leptos](https://crates.io/crates/leptos)                                               | ✅              | ⚠️ (Partial support)                      |
| Deployment config generation                                                                                                    | ❌              | ✅                                         |
| Starter templates                                                                                                               | ❌*             | ✅                                         |

<style>
table th:first-of-type {
    min-width: 75%;
}
table th:nth-of-type(2) {
    max-width: 10%;
}
table th:nth-of-type(3) {
    max-width: 10%;
}
</style>
