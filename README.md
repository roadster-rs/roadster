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
as [Rails](https://rubyonrails.org/), [Django](https://www.djangoproject.com/), [Laravel](https://laravel.com/),
and [Loco](https://github.com/loco-rs/loco).

## Features

- Built on Tokio's web stack (axum, tower, hyper, tracing). App behavior can be easily extended by taking advantage of
  all the resources in the tokio ecosystem.
- Built-in support for HTTP APIs via [Axum](https://docs.rs/axum) (with the `http` feature) and gRPC APIs
  via [Tonic](https://docs.rs/tonic) (with the `grpc` feature).
- Auto-generates an OpenAPI schema for HTTP API routes defined with [aide](https://docs.rs/aide) (requires
  the `open-api` feature).
- Support for running arbitrary long-running services (e.g., an API format not supported out of the box) with minimal
  boilerplate. Simply provide a
  [FunctionService](https://docs.rs/roadster/latest/roadster/service/function/service/struct.FunctionService.html)
  with your async function and register it in the `App#services` method.
- Provides sensible defaults so you can focus on building your app, but most (all?) of the built-in behavior can be
  customized or disabled via per-environment configuration files.
- Uses `#![forbid(unsafe_code)]` to ensure all code in Roadster is 100% safe rust.
- Provides a CLI for common commands, and allows consumers to provide their own CLI commands
  using [clap](https://docs.rs/clap) (requires the `cli` feature)
- Provides sample JWT extractor for Axum (requires the `jwt-ietf` and/or `jwt-openid` features). Also provides a general
  JWT extractor for Axum that simply puts all claims into a map (available with the `jwt` feature)
- Built-in support for [SeaORM](https://docs.rs/sea-orm), including creating DB connections (requires
  the `db-sea-orm` feature)
- Built-in support for [Diesel](https://docs.rs/diesel), including creating DB connections (requires a subset
  of the `db-diesel-*` collection of features, depending on what's needed)
- Built-in support for async workers backed by Postgres (via [pgmq](https://docs.rs/pgmq))
  or Redis/Sidekiq (via [rusty-sidekiq](https://docs.rs/rusty-sidekiq)). Requires the `worker-pg` or `worker-sidekiq`
  features,
  respectively.
- Built-in support for sending emails via SMTP (requires the `email-smtp` feature)
  or [Sendgrid's Mail Send API](https://www.twilio.com/docs/sendgrid/api-reference/mail-send/mail-send) (requires the
  `email-sendgrid` feature)
- Structured logs/traces using tokio's [tracing](https://docs.rs/tracing/latest/tracing/) crate. Export traces/metrics
  using OpenTelemetry (requires the `otel` feature).
- Health checks to ensure the app's external dependencies are healthy
- Pre-built migrations for common DB tables, e.g. `user` (requires the `db-sea-orm` feature)
- Support for auto-updating timestamp columns, e.g. `updated_at`, when updating DB rows (Postgres only currently) (
  requires the `db-sea-orm` feature)

A full list of features and their documentation can also be found in the [Roadster book](https://roadster.dev).

# Getting started

## Start local dependencies

Below are some example commands for running local instances of external dependencies, such as Postgres, Redis, and SMTP
servers.

### Database

```shell
# Replace `example_dev` with your app name, e.g., `myapp_dev`
docker run -d -p 5432:5432 -e POSTGRES_USER=roadster -e POSTGRES_DB=example_dev -e POSTGRES_PASSWORD=roadster postgres:18.0-alpine3.22
```

### Redis instance (for [Sidekiq.rs](https://docs.rs/rusty-sidekiq))

```shell
docker run -d -p 6379:6379 redis:8.2.2-alpine
```

### SMTP server

#### [Mailpit](https://github.com/axllent/mailpit)

```shell
docker run -d -p 8025:8025 -p 1025:1025 axllent/mailpit
```

#### [smtp4dev](https://github.com/rnwood/smtp4dev)

```shell
docker run -d -p 1080:80 -p 1025:25 rnwood/smtp4dev
```

#### [maildev](https://github.com/maildev/maildev)

```shell
docker run -d -p 1080:1080 -p 1025:1025 maildev/maildev
```

## Create your app

<!-- Todo: Add instructions for creating a new app -->

```shell
# Using one of our examples for now 
git clone https://github.com/roadster-rs/roadster.git
cd roadster/examples/app-builder
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
/ etc) or using a Rust front-end
framework ([Leptos](https://github.com/leptos-rs/leptos) / [Yew](https://github.com/yewstack/yew) / [Perseus](https://github.com/framesurge/perseus/) / [Sycamore](https://github.com/sycamore-rs/sycamore)
/ etc). That said, we do have some examples of how to use Roadster with some these frameworks.

## Examples

| Framework                                     | Example                                                                             |
|-----------------------------------------------|-------------------------------------------------------------------------------------|
| [Leptos](https://github.com/leptos-rs/leptos) | [leptos-ssr](https://github.com/roadster-rs/roadster/tree/main/examples/leptos-ssr) |

# Tracing + OpenTelemetry

Roadster allows reporting traces and metrics using the [`tracing`](https://docs.rs/tracing/latest/tracing/) and
[opentelemetry-rust](https://github.com/open-telemetry/opentelemetry-rust) integrations. Enable the `otel` and/or
`otel-grpc` features and provide the URL of your OTLP exporter in order to report the OTEL trace/metric data to your
telemetry provider (e.g., Grafana, SigNoz, New Relic, Datadog, etc).

# Background/async job queue

Roadster provides built-in support for running async workers using either Postgres (via [pgmq](https://docs.rs/pgmq)) or
Redis/Sidekiq (via [rusty-sidekiq](https://docs.rs/rusty-sidekiq)) as the backing store. See
the [Background jobs chapter](https://roadster.dev/features/services/background-jobs/index.html) of the book for more
details.

## Inspecting the Sidekiq state

### Sidekiq dashboard

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

### Redis Insights

You can also inspect the Redis DB directly using [RedisInsight](https://redis.io/docs/connect/insight/).

```shell
# Linux docker commands
docker run -d --name redisinsight --network=host -p 5540:5540 redis/redisinsight:latest
# Mac docker commands -- todo: see if there's a command that will work on both mac and linux
# Use `host.docker.internal` as the host domain in redis insight (instead of `127.0.0.1`)
docker run -d --name redisinsight -p 5540:5540 redis/redisinsight:latest
```

# Learning more

## Book

The [Roadster book](https://roadster.dev) provides more details on how to use all of Roadster's features.

## Examples

We also provide several examples for how to configure and use Roadster's features. These can be found the [
examples](https://github.com/roadster-rs/roadster/tree/main/examples) directory of this repository.

## GitHub Discussions

If you have a question not answered in the book or the examples,
please [open a GitHub Discussion](https://github.com/roadster-rs/roadster/discussions) and we'll be happy to
help.
