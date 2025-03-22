# OpenTelemetry

If the `otel` feature is enabled, Roadster will initialize various [OpenTelemetry](https://opentelemetry.io/) resources
in the [`init_tracing`](https://docs.rs/roadster/latest/roadster/tracing/fn.init_tracing.html), which is used in the
default implementation of [
`App#init_tracing`](https://docs.rs/roadster/latest/roadster/app/trait.App.html#method.init_tracing). In particular,
Roadster will set the OTEL service name and version and configure trace and metrics exporters.

## Sample OTEL configuration

```toml
{{ #include ../../../examples/tracing/config/development/otel.toml}}
```

## View metrics and traces locally

You can also view traces locally using, for example, Jaeger, Grafana, or SigNoz.

### Jaeger

Probably the easiest way to view OpenTelemetry Traces locally is by
running [Jaeger](https://www.jaegertracing.io/docs/2.4/getting-started/). Jaeger only supports traces, however. To
visualize metrics, use one of the other options mentioned in this section.

1. Set `ROADSTER__TRACING__OTLP_ENDPOINT="http://localhost:4317"` in your `.env` file, or in
   your `config/development.toml` or `config/test.toml` configs as appropriate.
2. Run the following command:
    ```shell
   docker run --rm --name jaeger \
     -p 16686:16686 \
     -p 4317:4317 \
     -p 4318:4318 \
     -p 5778:5778 \
     -p 9411:9411 \
     jaegertracing/jaeger:2.4.0
   ```
3. Navigate to the UI, which is available at [localhost:16686](http://localhost:16686).

### Grafana

🛠️ todo 🛠️

### Signoz

Another option to view traces and metrics locally is to run [Signoz](https://signoz.io/docs/install/docker/).

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

## Docs.rs links

- [`Tracing` config struct](https://docs.rs/roadster/latest/roadster/config/tracing/struct.Tracing.html)
- [`opentelemetry` crate](https://docs.rs/opentelemetry)
- [`tracing_opentelemetry` crate](https://docs.rs/tracing-opentelemetry)
