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

<!-- todo: add an example -->
