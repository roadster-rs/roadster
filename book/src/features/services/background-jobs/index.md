# Background jobs

In virtually all apps, there exists some work that needs to be done outside of the "critical path" in order to provide
a quick and responsive experience to the user. For example, in mobile apps, the only work that (should) happen on the
main thread is updating the UI. Everything else, such as reading files from disk and fetching data from the server,
happens on a background thread.

In web apps (and API servers), the same principle applies. In general, APIs should do the minimal amount of work needed
in order to response to the user's (or other service's) API request, and everything else should be moved to some
background "process". There are many ways this can be done; for
example, [AWS SQS](https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/welcome.html), [GCP
Pub/Sub](https://cloud.google.com/pubsub/docs/overview),
[Sidekiq](https://github.com/sidekiq/sidekiq), [Faktory](https://github.com/contribsys/faktory),
[pgmq](https://docs.rs/pgmq), to name a few.

Roadster provides a `Worker` trait to encapsulate common functionality for handling async jobs, and an `Enqueuer` trait
to handle enqueueing jobs into the job queue backend. The job queue backend for a worker can be easily changed simply
by changing the `Enqueuer` associated type for a `Worker` implementation.

## Built-in worker backends

Roadster provides built-in support for running async workers using either Postgres (via [pgmq](https://docs.rs/pgmq)) or
Redis/Sidekiq (via [rusty-sidekiq](https://docs.rs/rusty-sidekiq)) as the backing store. See the following chapters for
more details on
each.

## Benchmarks

Roadster has a (small) [benchmark suite](https://github.com/roadster-rs/roadster/tree/main/benches/worker) to compare
the worker backends we support. Below is a link to an example run of the benchmark. The number in the benchmark name
indicates the number of worker tasks used to handle the jobs.

- [Benchmark run on an M3 Macbook Air](benchmarks/m3/report/index.html)
- [Benchmark run on an Arch Linux destkop with an AMD 5800X and 32GB RAM](benchmarks/AMD-5800X/report/index.html)

## Example

### Pg vs Sidekiq worker definition

Notice that the `Worker` implementation is identical for both a Postgres- vs a Sidekiq-backed queue. The only difference
is the `Enqueuer` associated type.

```rust,ignore
{{#include ../../../../examples/service/src/worker/pg/worker.rs:11:14}}
{{#include ../../../../examples/service/src/worker/pg/worker.rs:24:}}
```

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/worker.rs:11:14}}
{{#include ../../../../examples/service/src/worker/sidekiq/worker.rs:24:}}
```

### Pg vs Sidekiq worker registration

Workers need to be registered with a queue processor. The processor should at least be registered with the processor
that matches its `Enqueuer`. However, it can also be registered with a processor that's different from its `Enqueuer`.
This is useful if a worker's `Enqueuer` needs to change -- in this case, it's possible for some jobs to remain
in the old backend after the `Enqueuer` was switched. To ensure jobs in the old backend are processed, the worker can
temporarily be registered with both the old and new processors, and once all the jobs in the old backend are completed,
the worker can be removed from the old processor.

The built-in Postgres and Sidekiq processors have the same APIs, so migrating between the two is easy.

```rust,ignore
{{#include ../../../../examples/service/src/worker/pg/register.rs:12:17}}
{{#include ../../../../examples/service/src/worker/pg/register.rs:26:32}}
```

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/register.rs:12:17}}
{{#include ../../../../examples/service/src/worker/sidekiq/register.rs:26:33}}
```

### Enqueuing jobs

Enqueueing jobs from the application code is identical between each type of queue backend.

```rust,ignore
{{#include ../../../../examples/service/src/worker/sidekiq/enqueue.rs:7:}}
```
