#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
// Ignore the warning that the `coverage_nightly` cfg is not recognized.
#![cfg_attr(test, allow(unexpected_cfgs))]
#![cfg_attr(rustc_unstable, feature(coverage_attribute))]

pub mod api;
pub mod app;
pub mod config;
#[cfg(feature = "db-sql")]
pub mod db;
pub mod error;
pub mod health;
pub mod lifecycle;
pub mod middleware;
pub mod service;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tracing;
pub mod util;
#[cfg(feature = "worker")]
pub mod worker;
