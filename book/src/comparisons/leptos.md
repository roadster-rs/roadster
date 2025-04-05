# Roadster vs. [Leptos](https://crates.io/crates/leptos)

Leptos is a reactive UI web framework for Rust. However, it has a concept of "server functions", an abstraction for
calling server-side logic from either the frontend or the backend. Leptos leaves it up to the consumer to set up
the backend using Axum or Actix. That's where Roaster comes in -- Roadster takes care of
configuring all the backend resources you need using Axum as the HTTP router. So, Roadster and Leptos can be used
together to easily build your full-stack web application fully in Rust.

See the following example(s) for how to use Roadster with Leptos:

- [leptos-ssr](https://github.com/roadster-rs/roadster/tree/main/examples/leptos-ssr)
