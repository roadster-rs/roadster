# Observability

Observability is an important component of any web service in order to monitor the health of the system and investigate
when things go wrong. Observability includes things such as traces (or logs) and metrics. Roadster recommends emitting
traces using Tokio's [`tracing` crate](https://docs.rs/tracing/latest/tracing/) and provides a default tracing
configuration. If the `otel` feature is enabled, Roadster also supports exporting traces and metrics
via [OpenTelemetry](https://opentelemetry.io/), which enables viewing traces and metrics in any observability platform
that supports OpenTelemetry, such as Grafana or SigNoz.
