# Roadster

![github ci](https://github.com/roadster-rs/roadster/actions/workflows/ci.yml/badge.svg?branch=main)

A "Batteries Included" web framework for rust designed to get you moving fast 🏎️. Inspired by other fully-featured
frameworks such as [Rails](https://rubyonrails.org/), [Loco](https://github.com/loco-rs/loco),
and [Poem](https://github.com/poem-web/poem).

## Features

- Built on Tokio's web stack (axum, tower, hyper, tracing). App behavior can be easily extended by taking advantage of
  all the resources in the tokio ecosystem.
- Provides sane defaults so you can focus on building your app.
- Most of the built-in behavior can be customized or even disabled via per-environment configuration files.
- Uses `#![forbid(unsafe_code)]` to ensure all code in Roadster is 100% safe rust.

# Start local DB

```shell
# Dev
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
# Test
docker run -d -p 5433:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=myapp_test -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
```

# Start local Redis instance (for [Sidekiq.rs](https://crates.io/crates/rusty-sidekiq))

```shell
# Dev
docker run -d -p 6379:6379 redis:7.2-alpine
# Test
docker run -d -p 6380:6379 redis:7.2-alpine
```

# Tracing (via OpenTelemetry)

Roadster allows reporting traces and metrics using the `tracing` and `opentelemetry_rust` integrations. Provide the URL
of your OTLP exporter in order to report the trace/metric data to your telemetry provider (e.g., SigNoz, New Relic,
Datadog, etc).

## View traces locally

You can also view traces locally using, for example, Jaeger or SigNoz.

### Jaeger

The easiest way to view OpenTelemetry Traces locally is by
running [Jaeger](https://www.jaegertracing.io/docs/1.54/getting-started/).

1. Set `ROADSTER.TRACING.OTLP_ENDPOINT="http://localhost:4317"` in your `.env` file, or in
   your `config/development.toml` or `config/test.toml` configs as appropriate.
2. Run the following command:
    ```shell
    docker run --rm --name jaeger \
        -e COLLECTOR_ZIPKIN_HOST_PORT=:9411 \
        -p 6831:6831/udp \
        -p 6832:6832/udp \
        -p 5778:5778 \
        -p 16686:16686 \
        -p 4317:4317 \
        -p 4318:4318 \
        -p 14250:14250 \
        -p 14268:14268 \
        -p 14269:14269 \
        -p 9411:9411 \
        jaegertracing/all-in-one:1.53
    ```
3. Navigate to the UI, which is available at [localhost:16686](http://localhost:16686).

### Signoz

Another option to view traces (and metrics) locally is to run [Signoz](https://signoz.io/docs/install/docker/).

1. Set `ROADSTER.TRACING.OTLP_ENDPOINT="http://localhost:4317"` in your `.env` file, or in
   your `config/development.toml` or `config/test.toml` configs as appropriate.
2. Install and run Signoz in a directory of your choice
   ```shell
   cd ~/code/
   # Clone the repo
   git clone -b main https://github.com/SigNoz/signoz.git && cd signoz/deploy/
   # Remove the sample application: https://signoz.io/docs/operate/docker-standalone/#remove-the-sample-application-from-signoz-dashboard
   vim docker/clickhouse-setup/docker-compose.yaml
   # Remove the `services.hotrod` and `services.load-hotrod` sections, then exit `vim`
   # Run the `docker compose` command
   ./install.sh
   ```
3. Navigate to the UI, which is available at [localhost:3301](http://localhost:3301).
4. To stop Signoz, run the following:
   ```shell
   docker compose -f docker/clickhouse-setup/docker-compose.yaml stop
   ```

# Background/async job queue

There are a few different ways we can implement background/async jobs (currently only Sidekiq.rs is supported).

## [Sidekiq.rs](https://crates.io/crates/rusty-sidekiq)

This crate is a rust implementation of [Sidekiq](https://sidekiq.org/), which is usually used with Ruby on Rails. All we
need
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

## [Apalis](https://crates.io/crates/apalis)

Todo: [Evaluate using this](https://github.com/MassDissent/roadster/issues/3)

## External/managed queues

Todo: Evaluate using these instead of a self-hosted queue.

- Kafka queue
- SQS
- Pub/Sub (e.g., Google Cloud's offering)

# Development

## Code Coverage

```shell
# Install `binstall` (or use the normal `cargo install` command below instead
curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
# Install coverage dependencies
cargo binstall grcov
rustup component add llvm-tools
# Build + run tests with coverage
RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="coverage/$USER-%p-%m.profraw" cargo build
RUSTFLAGS="-Cinstrument-coverage" LLVM_PROFILE_FILE="coverage/$USER-%p-%m.profraw" cargo test
# Generate and open an HTML coverage report
grcov . -s . --binary-path ./target/debug/ -t html --branch --ignore-not-existing -o ./target/debug/coverage/
open target/debug/coverage/index.html
# Delete the *.profraw files generated by the coverage tooling
rm coverage/$USER-*.profraw
```
