//! Background task queue service backed by Redis using [rusty-sidekiq](https://docs.rs/rusty-sidekiq).

pub mod app_worker;
pub mod builder;
pub(crate) mod processor_wrapper;
pub mod roadster_worker;
pub mod service;
