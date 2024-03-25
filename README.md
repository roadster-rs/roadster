# Roadster

# Start local DB

```shell
# Dev
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
# Test
docker run -d -p 5433:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_test -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
```

# Start local Redis instance (for Sidekiq.rs ([rusty-sidekiq](https://crates.io/crates/rusty-sidekiq)))

```shell
# Dev
docker run -d -p 6379:6379 redis:7.2-alpine
# Test
docker run -d -p 6380:6379 redis:7.2-alpine
```

# Background/async job queue

There are a few different ways we can implement background/async jobs.

## Sidekiq.rs ([rusty-sidekiq](https://crates.io/crates/rusty-sidekiq))

This crate a rust implementation of [Sidekiq](https://sidekiq.org/) that's usually used with Ruby on Rails. All we need
to use this is a Redis instance.

## [Faktory](https://crates.io/crates/faktory)

This crate integrates with Faktory, a language agnostic job queue from the creators of Sidekiq. Unfortunately, this
crate is not quite ready to be used in production. Some reasons why we're not using this crate:

- No async support in job handlers
- Not able to signal consumers to gracefully shut down
- Job handlers can't use `anyhow`/`eyre` and instead need to use an `Error` type that implements `std::error::Error`.
  This
  can fairly easily be done using [thiserror](https://crates.io/crates/thiserror) to wrap `anyhow::Error`, but it's
  still not ideal.
    ```rust
    // Example wrapping `anyhow::Error`
    #[derive(Error, Debug)]
    pub enum AnyhowWrapper {
        #[error(transparent)]
        Other(#[from] anyhow::Error),
    }
    
    ```
- The provided methods of enqueuing jobs decouple the queue name from the job handler. We would need (want) to create a
  custom method of enqueueing jobs that automates providing the correct queue name.

## External/managed queues

Todo: Evaluate using these instead of a self-hosted queue.

- Kafka queue
- SQS
- Pub/Sub (e.g., Google Cloud's offering)