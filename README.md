# Roadster

[![crates.io](https://img.shields.io/crates/v/roadster.svg)](https://crates.io/crates/roadster)
[![docs.rs](https://img.shields.io/docsrs/roadster?logo=docsdotrs)](https://docs.rs/roadster/latest/roadster/)
[![Crates.io MSRV](https://img.shields.io/crates/msrv/roadster)](https://crates.io/crates/roadster)
[![Crates.io License](https://img.shields.io/crates/l/roadster)](https://crates.io/crates/roadster)
[![GitHub Discussions](https://img.shields.io/github/discussions/roadster-rs/roadster?logo=github)](https://github.com/roadster-rs/roadster/discussions)
[![codecov](https://codecov.io/gh/roadster-rs/roadster/graph/badge.svg?token=JIMN3U8K88)](https://codecov.io/gh/roadster-rs/roadster)
[![Checks](https://github.com/roadster-rs/roadster/actions/workflows/ci.yml/badge.svg)](https://github.com/roadster-rs/roadster/actions/workflows/ci.yml)
[![Feature Powerset](https://github.com/roadster-rs/roadster/actions/workflows/feature_powerset.yml/badge.svg)](https://github.com/roadster-rs/roadster/actions/workflows/feature_powerset.yml)
[![Book](https://github.com/roadster-rs/roadster/actions/workflows/book.yml/badge.svg)](https://github.com/roadster-rs/roadster/actions/workflows/book.yml)
[![dependency status](https://deps.rs/crate/roadster/latest/status.svg)](https://deps.rs/crate/roadster/)

A "Batteries Included" web framework for rust designed to get you moving fast 🏎️. Inspired by other fully-featured
frameworks such
as [Rails](https://rubyonrails.org/), [Django](https://www.djangoproject.com/), [Laravel](https://laravel.com/), [Loco](https://github.com/loco-rs/loco),
and [Poem](https://github.com/poem-web/poem).

## Features

- Built on Tokio's web stack (axum, tower, hyper, tracing). App behavior can be easily extended by taking advantage of
  all the resources in the tokio ecosystem.
- Built-in support for HTTP APIs via [Axum](https://crates.io/crates/axum) (with the `http` feature) and gRPC APIs
  via [Tonic](https://crates.io/crates/tonic) (with the `grpc` feature).
- Auto-generates an OpenAPI schema for HTTP API routes defined with [aide](https://crates.io/crates/aide) (requires
  the `open-api` feature).
- Support for running arbitrary long-running services (e.g., an API format not supported out of the box) with minimal
  boilerplate. Simply provide a
  [FunctionService](https://docs.rs/roadster/latest/roadster/service/function/service/struct.FunctionService.html)
  with your async function and register it in the `App#services` method.
- Provides sensible defaults so you can focus on building your app, but most (all?) of the built-in behavior can be
  customized or disabled via per-environment configuration files.
- Uses `#![forbid(unsafe_code)]` to ensure all code in Roadster is 100% safe rust.
- Provides a CLI for common commands, and allows consumers to provide their own CLI commands
  using [clap](https://crates.io/crates/clap) (requires the `cli` feature)
- Provides sample JWT extractor for Axum (requires the `jwt-ietf` and/or `jwt-openid` features). Also provides a general
  JWT extractor for Axum that simply puts all claims into a map (available with the `jwt` feature)
- Built-in support for [SeaORM](https://crates.io/crates/sea-orm), including creating DB connections (requires
  the `db-sea-orm` feature)
- Built-in support for [Sidekiq.rs](https://crates.io/crates/rusty-sidekiq) for running async/background jobs (requires
  the `sidekiq` feature)
- Built-in support for sending emails via SMTP (requires the `email-smtp` feature)
  or [Sendgrid's Mail Send API](https://www.twilio.com/docs/sendgrid/api-reference/mail-send/mail-send) (requires the
  `email-sendgrid` feature)
- Structured logs/traces using tokio's [tracing](https://docs.rs/tracing/latest/tracing/) crate. Export traces/metrics
  using OpenTelemetry (requires the `otel` feature).
- Health checks to ensure the app's external dependencies are healthy
- Pre-built migrations for common DB tables, e.g. `user` (requires the `db-sea-orm` feature)
- Support for auto-updating timestamp columns, e.g. `updated_at`, when updating DB rows (Postgres only currently) (
  requires the `db-sea-orm` feature)

# Getting started

## Start local DB

```shell
# Replace `example_dev` with your app name, e.g., `myapp_dev`
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=example_dev -e POSTGRES_PASSWORD=roadster postgres:15.3-alpine
```

## Start local Redis instance (for [Sidekiq.rs](https://crates.io/crates/rusty-sidekiq))

```shell
docker run -d -p 6379:6379 redis:7.2-alpine
```

## Start local SMTP server instance

### [Mailpit](https://github.com/axllent/mailpit)

```shell
docker run -d -p 8025:8025 -p 1025:1025 axllent/mailpit
```

### [smtp4dev](https://github.com/rnwood/smtp4dev)

```shell
docker run -d -p 1080:80 -p 1025:25 rnwood/smtp4dev
```

### [maildev](https://github.com/maildev/maildev)

```shell
docker run -d -p 1080:1080 -p 1025:1025 maildev/maildev
```

## Create your app

```shell
# Todo: Add instructions for creating a new app
# Using one of our examples for now 
git clone https://github.com/roadster-rs/roadster.git
cd roadster/examples/full
```

## Set the environment (production/development/test)

```shell
# Either set it as an environment variable
export ROADSTER__ENVIRONMENT=development
# Or add it to a `.env` file
echo ROADSTER__ENVIRONMENT=development >> .env
```

## Start your app

```shell
cargo run
```

## Explore the API

Navigate to http://localhost:3000/api/_docs to explore the app's OpenAPI playground

# Add a UI

Currently, Roadster is focused on back-end API development with Rust. We leave it to the consumer to decide how they
prefer to add a front-end, e.g., using an established JS/TS
framework ([React](https://react.dev/) / [Next](https://nextjs.org/) / [Vue](https://vuejs.org/) / [Svelte](https://svelte.dev/) / [Solid](https://www.solidjs.com/)
/ etc) or
using a Rust front-end
framework ([Leptos](https://github.com/leptos-rs/leptos) / [Yew](https://github.com/yewstack/yew) / [Perseus](https://github.com/framesurge/perseus/) / [Sycamore](https://github.com/sycamore-rs/sycamore)
/ etc). That said, we do have some examples of how to use Roadster with some these frameworks.

## Examples

| Framework                                     | Example                                                                             |
|-----------------------------------------------|-------------------------------------------------------------------------------------|
| [Leptos](https://github.com/leptos-rs/leptos) | [leptos-ssr](https://github.com/roadster-rs/roadster/tree/main/examples/leptos-ssr) |

# Email

## Local testing of sending emails via SMTP

If you're using our SMTP integration to send emails, you can test locally using a mock SMTP server. Some options:

- [maildev](https://github.com/maildev/maildev)
- [smtp4dev](https://github.com/rnwood/smtp4dev)

# Tracing + OpenTelemetry

Roadster allows reporting traces and metrics using the `tracing` and `opentelemetry_rust` integrations. Provide the URL
of your OTLP exporter in order to report the trace/metric data to your telemetry provider (e.g., SigNoz, New Relic,
Datadog, etc).

## View traces locally

You can also view traces locally using, for example, Jaeger or SigNoz.

### Jaeger

The easiest way to view OpenTelemetry Traces locally is by
running [Jaeger](https://www.jaegertracing.io/docs/1.54/getting-started/).

1. Set `ROADSTER__TRACING__OTLP_ENDPOINT="http://localhost:4317"` in your `.env` file, or in
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

1. Set `ROADSTER__TRACING__OTLP_ENDPOINT="http://localhost:4317"` in your `.env` file, or in
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
# Use `host.docker.internal` as the host domain in redis insight (instead of `127.0.0.1`)
docker run -d --name redisinsight -p 5540:5540 redis/redisinsight:latest
```
