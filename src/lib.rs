//! A "Batteries Included" web framework for rust designed to get you moving fast 🏎️. Inspired by other fully-featured
//! frameworks such as [Rails](https://rubyonrails.org/), [Loco](https://github.com/loco-rs/loco),
//! and [Poem](https://github.com/poem-web/poem).
//!
//! ## Features
//!
//! - Built on Tokio's web stack (axum, tower, hyper, tracing). App behavior can be easily extended by taking advantage of
//!   all the resources in the tokio ecosystem.
//! - Provides sane defaults so you can focus on building your app.
//! - Most of the built-in behavior can be customized or even disabled via per-environment configuration files.
//! - Uses `#![forbid(unsafe_code)]` to ensure all code in Roadster is 100% safe rust.

#![forbid(unsafe_code)]
#![cfg_attr(coverage, feature(coverage_attribute))]

pub mod app;
pub mod app_context;
pub mod auth;
#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
pub mod controller;
pub mod service;
pub mod tracing;
pub mod util;
pub mod view;
