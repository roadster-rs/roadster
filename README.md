# Roadster

[![Checks](https://github.com/roadster-rs/roadster/actions/workflows/ci.yml/badge.svg)](https://github.com/roadster-rs/roadster/actions/workflows/ci.yml)
[![Feature Powerset](https://github.com/roadster-rs/roadster/actions/workflows/feature_powerset.yml/badge.svg)](https://github.com/roadster-rs/roadster/actions/workflows/feature_powerset.yml)

A "Batteries Included" web framework for rust designed to get you moving fast 🏎️. Inspired by other fully-featured
frameworks such as [Rails](https://rubyonrails.org/), [Loco](https://github.com/loco-rs/loco),
and [Poem](https://github.com/poem-web/poem).

## Features

- Built on Tokio's web stack (axum, tower, hyper, tracing). App behavior can be easily extended by taking advantage of
  all the resources in the tokio ecosystem.
- Provides sane defaults so you can focus on building your app.
- Most of the built-in behavior can be customized or even disabled via per-environment configuration files.
- Uses `#![forbid(unsafe_code)]` to ensure all code in Roadster is 100% safe rust.
- Auto-generates an OpenAPI schema for routes defined with [aide](https://crates.io/crates/aide) (requires
  the `open-api` feature)
- Provides a CLI for common commands, and allows consumers to provide their own CLI commands
  using [clap](https://crates.io/crates/clap) (requires the `cli` feature)
- Provides sample JWT extractor for Axum (requires the `jwt-ietf` and/or `jwt-openid` features). Also provides a general
  JWT extractor for Axum that simply puts all claims into a map (available with the `jwt` feature)
- Built-in support for [SeaORM](https://crates.io/crates/sea-orm), including creating DB connections (requires
  the `db-sql` feature)
- Built-in support for [Sidekiq.rs](https://crates.io/crates/rusty-sidekiq) for running async/background jobs (requires
  the `sidekiq` feature)
- Export traces/metrics using OpenTelemetry (requires the `otel` feature)

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

# Background/async job queue using [Sidekiq.rs](https://crates.io/crates/rusty-sidekiq)

This crate is a rust implementation of [Sidekiq](https://sidekiq.org/), which is usually used with Ruby on Rails. All we
need in order to use this is a Redis instance.

## Sidekiq dashboard

We provide a [sample repo](https://github.com/roadster-rs/standalone_sidekiq_dashboard) to run the sidekiq dashboard
locally in a standalone docker container.

```shell
git clone https://github.com/roadster-rs/standalone_sidekiq_dashboard.git
cd standalone_sidekiq_dashboard
docker build -t standalone-sidekiq .
# Linux docker commands
# Development
docker run -d --network=host standalone-sidekiq
# Test
docker run -d --network=host -e REDIS_URL='redis://localhost:6380' standalone-sidekiq

# Mac docker commands -- todo: see if there's a command that will work on both mac and linux
# Development
docker run -d -p 9292:9292 -e REDIS_URL=redis://host.docker.internal:6379 standalone-sidekiq
# Test
docker run -d -p 9292:9292 -e REDIS_URL=redis://host.docker.internal:6380 standalone-sidekiq
```

## Redis Insights

You can also inspect the Redis DB directly using [RedisInsight](https://redis.io/docs/connect/insight/).

```shell
# Linux docker commands
docker run -d --name redisinsight --network=host -p 5540:5540 redis/redisinsight:latest
# Mac docker commands -- todo: see if there's a command that will work on both mac and linux
docker run -d --name redisinsight -p 5540:5540 redis/redisinsight:latest
```

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
