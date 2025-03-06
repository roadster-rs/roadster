# Background jobs with Sidekiq

[Sidekiq](https://github.com/sidekiq/sidekiq) is a popular system for running background and cron jobs in Ruby on Rails
apps. Roadster provides built-in support for running background jobs with [Sidekiq](https://github.com/sidekiq/sidekiq)
via the [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/) crate, which provides a Rust interface for
interacting with a Sidekiq server (e.g., a Redis server).

Below is an example of how to register a worker and enqueue it into the job queue. See
the [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/) for more details on implementing [
`Worker`](https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html)s.

## Service configs

See: <https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.SidekiqServiceConfig.html>

🛠 todo 🛠

## Worker configs

See: <https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/app_worker/struct.AppWorkerConfig.html>

🛠 todo 🛠

## Example

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/mod.rs:11:}}
```

## Docs.rs links

- [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/)
- [Roadster `sidekiq` mod](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/index.html)
- [SidekiqServiceConfig](https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.SidekiqServiceConfig.html)
- [AppWorkerConfig](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/app_worker/struct.AppWorkerConfig.html)
