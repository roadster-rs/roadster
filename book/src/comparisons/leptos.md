# Roadster vs. [Leptos](https://crates.io/crates/leptos)

Leptos is a full-stack web framework for Rust. However, it mainly focuses on the UI side of things and leaves it up to
the consumer to set up the backend using Axum or Actix. That's where Roaster comes in -- Roadster takes care of
configuring all the backend resources you need using Axum as the HTTP router. So, Roadster and Leptos can be used
together to easily build your full-stack web application fully in Rust.

For an example of how to use Leptos with Roadster, see our Leptos examples:

- [leptos-0.7-ssr](https://github.com/roadster-rs/roadster/tree/main/examples/leptos-0.7-ssr) - Leptos 0.7 example
- [leptos-ssr](https://github.com/roadster-rs/roadster/tree/main/examples/leptos-ssr) - Leptos 0.6 example
