# Introduction

> ***This book is a work in progress. If you can't find the information you need here, check
> the [doc.rs documentation](https://docs.rs/roadster/latest/roadster/)
> or [open a GitHub discussion](https://github.com/roadster-rs/roadster/discussions/new/choose).***

This book is intended as a guide for how to use [Roadster](https://crates.io/crates/roadster), a "batteries included"
web framework for Rust designed to get you moving fast 🏎️. Compared to low-level web frameworks such
as [Axum](https://github.com/tokio-rs/axum) or [Actix](https://actix.rs/), which only provide the functionality to
create an API, Roadster aims to provide all the other functionality needed to create a fully-featured backend or
fullstack web app for a more "batteries included"experience. Roadster is designed to provide sensible defaults for all
features while remaining highly configurable, customizable, and pluggable. This allows you to focus on creating your
application instead of wiring up all of your dependencies, while still allowing you the flexibility to customize your
dependencies if needed.

If you're unsure if Roadster is the best fit for your project, a collection of comparisons to other Rust web frameworks
can be found in [Web framework comparisons](comparisons/index.md). The full list of Roadster's features can be
found in [Roadster features](features/index.md).

## Prerequisite reading

This book assumes the reader has some knowledge of how to program in Rust. If you are new
to Rust, you may find the following helpful to better understand how to use Roadster:

- [The Rust Programming Language](https://doc.rust-lang.org/book/)
- [Rust By Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust website](https://www.rust-lang.org/learn)

In addition, as asynchronous programming is essential for writing performant web apps, some knowledge of async
programming in Rust is assumed. If you are unfamiliar with async Rust, you may find the following helpful:

- [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
    - Tokio is one of the two most popular async runtimes (the other
      being [async-std](https://docs.rs/async-)). Roadster only supports Tokio at the moment.