# Background jobs with Sidekiq

[Sidekiq](https://github.com/sidekiq/sidekiq) is a popular system for running background and cron jobs in Ruby on Rails
apps. Roadster provides built-in support for running background jobs with [Sidekiq](https://github.com/sidekiq/sidekiq)
via the [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/) crate, which provides a Rust interface for
interacting with a Sidekiq server (e.g., a Redis server).

Below is an example of how to register a worker and enqueue it into the job queue.

## Service configs

Various properties of the Sidekiq worker service can be configured via the app's config files. The most important fields
to configure are the following:

- `service.worker.sidekiq.num-workers`: The number of Sidekiq worker tasks that can run at the same time.
- `service.worker.sidekiq.queues`: The names of the worker queues to handle.
- `service.worker.sidekiq.redis.uri`: The URI of the Redis database to use as the Sidekiq server.

```toml
{{ #include ../../../../examples/service/config/development/sidekiq.toml }}
```

See
the [config struct](https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.SidekiqWorkerServiceConfig.html)
for the full list of fields available.

## Worker configs

In addition to the service-level configs, each worker has various configurable values that can be provided
by implementing the `Worker::worker_config` method. Any configs not provided in this implementation will fall back
to the values provided in the app config.

```toml
{{ #include ../../../../examples/service/config/development/worker.toml }}
```

## Example

### Worker definition

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/worker.rs:10:}}
```

### Register the worker with the processor

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/register.rs:8:}}
```

### Enqueue a job for the worker

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/enqueue.rs:7:}}
```

## Docs.rs links

- [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/)
- [Roadster `sidekiq` mod](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/index.html)
- [SidekiqServiceConfig](https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.SidekiqServiceConfig.html)
- [AppWorkerConfig](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/app_worker/struct.AppWorkerConfig.html)
