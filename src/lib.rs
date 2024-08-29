#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
// Ignore the warning that the `coverage_nightly` cfg is not recognized.
#![cfg_attr(test, allow(unexpected_cfgs))]
// https://github.com/taiki-e/coverage-helper?tab=readme-ov-file#usage
#![cfg_attr(all(test, coverage_nightly), feature(coverage_attribute))]

pub mod api;
pub mod app;
pub mod config;
pub mod error;
pub mod health_check;
pub mod lifecycle;
pub mod middleware;
#[cfg(feature = "db-sql")]
pub mod migration;
pub mod service;
#[cfg(any(test, feature = "testing"))]
pub mod testing;
pub mod tracing;
pub mod util;
