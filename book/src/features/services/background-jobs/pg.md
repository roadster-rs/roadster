# Background jobs with Sidekiq

Roadster provides built-in support for running background jobs using Postgres as the queue backing store. Roadster
uses the Rust-only integration provided by [pgmq](https://docs.rs/pgmq/) (as opposed to the Postgres extension
integration). This means that all Roadster needs in order to provide a Postgres-backed queue is a URI for a standard
Postgres DB with no extensions required.

Below is an example of how to register a worker and enqueue it into the job queue.

## Service configs

Various properties of the Postgres worker service can be configured via the app's config files. The most important
fields to configure are the following:

- `service.worker.pg.num-workers`: The number of worker tasks that can run at the same time.
- `service.worker.pg.queues`: The names of the worker queues to handle.
- `service.worker.pg.database.uri`: The URI of the Postgres database to use to enqueue jobs. If not provided, will fall
  back to the value in the `database.uri` config field.

```toml
{{ #include ../../../../examples/service/config/development/pg_worker.toml }}
```

See
the [config struct](https://docs.rs/roadster/latest/roadster/config/service/worker/pg/struct.PgWorkerServiceConfig.html)
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
{{#include ../../../../examples/service/src/worker/pg/worker.rs:10:}}
```

### Register the worker with the processor

```rust,ignore
{{#include ../../../../examples/service/src/worker/pg/register.rs:8:}}
```

### Enqueue a job for the worker

```rust,ignore
{{#include ../../../../examples/service/src/worker/pg/enqueue.rs:7:}}
```

## Docs.rs links

- [pgmq](https://docs.rs/pgmq/)
- [Roadster pg worker mod](https://docs.rs/roadster/latest/roadster/worker/backend/pg/index.html)
