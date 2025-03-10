# Background jobs with Sidekiq

[Sidekiq](https://github.com/sidekiq/sidekiq) is a popular system for running background and cron jobs in Ruby on Rails
apps. Roadster provides built-in support for running background jobs with [Sidekiq](https://github.com/sidekiq/sidekiq)
via the [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/) crate, which provides a Rust interface for
interacting with a Sidekiq server (e.g., a Redis server).

Below is an example of how to register a worker and enqueue it into the job queue. See
the [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/) for more details on implementing [
`Worker`](https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html)s.

## Service configs

Various properties of the Sidekiq worker service can be configured via the app's config files. The most important fields
to configure are the following:

- `service.sidekiq.num-workers`: The number of Sidekiq workers that can run at the same time.
- `service.sidekiq.queues`: The names of the worker queues to handle.
- `service.sidekiq.redis.uri`: The URI of the Redis database to use as the Sidekiq server.

```toml
{{ #include ../../../../examples/service/config/development/sidekiq.toml }}
```

See
the [config struct](https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.SidekiqServiceConfig.html)
for the full list of fields available.

## Worker configs

In addition to the service-level configs, each worker has various configurable values. Some of these can be provided
by implementing the respective methods of the
sidekiq.rs [Worker](https://docs.rs/rusty-sidekiq/latest/sidekiq/trait.Worker.html) trait. However, they can also
be provided when the worker is registered with
the [SidekiqWorkerServiceBuilder](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/builder/struct.SidekiqWorkerServiceBuilder.html).

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/worker_with_configs.rs:13:20}}
```

## Roadster worker

All workers registered with
the [SidekiqWorkerServiceBuilder](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/builder/struct.SidekiqWorkerServiceBuilder.html)
are wrapped in our
custom [RoadsterWorker](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/roadster_worker/struct.RoadsterWorker.html).
This allows us to implement some additional features for workers. Specifically, the ability to set a max duration for
workers, after which they will automatically timeout, be reported as an error, and be retried according to the
worker's retry config. The default behavior is to timeout after `60` seconds, but this can be extended or disabled at
the service level or in each individual worker.

Note: in order for a worker to stop running when the timeout is exceeded, the worker needs to hit an `await` point.
So, it will work great for async IO-bound tasks, but CPU-bound tasks will require manual yields (e.g.
with [yield_now](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html)) in order for the tasks to be automatically
timed out.

## Example

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/mod.rs:14:}}
```

## Docs.rs links

- [Sidekiq.rs](https://docs.rs/rusty-sidekiq/latest/sidekiq/)
- [Roadster `sidekiq` mod](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/index.html)
- [SidekiqServiceConfig](https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.SidekiqServiceConfig.html)
- [AppWorkerConfig](https://docs.rs/roadster/latest/roadster/service/worker/sidekiq/app_worker/struct.AppWorkerConfig.html)
