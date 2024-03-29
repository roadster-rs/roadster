//! # Roadster
//!
//! Todo: Add documentation

#![forbid(unsafe_code)]

pub mod app;
pub mod app_context;
pub mod auth;
pub mod config;
pub mod controller;
pub mod initializer;
pub mod tracing;
pub mod util;
pub mod view;
#[cfg(feature = "sidekiq")]
pub mod worker;
