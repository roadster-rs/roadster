use crate::app::metadata::AppMetadata;
use crate::config::AppConfig;
use crate::config::tracing::Format;
use crate::error::RoadsterResult;
#[cfg(feature = "otel")]
use convert_case::{Case, Casing};
#[cfg(feature = "otel")]
use opentelemetry::trace::TracerProvider;
#[cfg(feature = "otel")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "otel")]
use opentelemetry_otlp::{MetricExporter, SpanExporter};
#[cfg(feature = "otel")]
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
#[cfg(feature = "otel")]
use opentelemetry_sdk::propagation::TraceContextPropagator;
#[cfg(feature = "otel")]
use opentelemetry_sdk::trace::Sampler;
#[cfg(feature = "otel")]
use opentelemetry_semantic_conventions::resource::SERVICE_VERSION;
use std::str::FromStr;
use tracing::Level;
#[cfg(feature = "otel")]
use tracing_opentelemetry::MetricsLayer;
#[cfg(feature = "otel")]
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracing(
    config: &AppConfig,
    #[allow(unused_variables)] // This parameter isn't used in some feature combinations
    metadata: &AppMetadata,
) -> RoadsterResult<()> {
    // Stdout Layer
    // Each format results in a different type, so we can't use a `match` on the format enum.
    // Instead, we need to create an optional layer of each type, and add all of them to the
    // registry -- if a layer is `None`, it won't actually be added.
    let compact_log_layer = if matches!(config.tracing.format, Format::Compact) {
        Some(tracing_subscriber::fmt::layer().compact())
    } else {
        None
    };
    let pretty_log_layer = if matches!(config.tracing.format, Format::Pretty) {
        Some(tracing_subscriber::fmt::layer().pretty())
    } else {
        None
    };
    let json_log_layer = if matches!(config.tracing.format, Format::Json) {
        Some(tracing_subscriber::fmt::layer().json())
    } else {
        None
    };
    match config.tracing.format {
        Format::None => {
            assert!(
                pretty_log_layer.is_none()
                    && compact_log_layer.is_none()
                    && json_log_layer.is_none()
            )
        }
        Format::Pretty => {
            assert!(
                pretty_log_layer.is_some()
                    && compact_log_layer.is_none()
                    && json_log_layer.is_none()
            )
        }
        Format::Compact => {
            assert!(
                compact_log_layer.is_some()
                    && pretty_log_layer.is_none()
                    && json_log_layer.is_none()
            )
        }
        Format::Json => {
            assert!(
                json_log_layer.is_some()
                    && pretty_log_layer.is_none()
                    && compact_log_layer.is_none()
            )
        }
    }

    #[cfg(feature = "otel")]
    if config.tracing.trace_propagation {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    }

    #[cfg(feature = "otel")]
    let service_name = config
        .tracing
        .service_name
        .clone()
        .or(metadata.name.clone())
        .unwrap_or(config.app.name.to_case(Case::Snake));

    #[cfg(feature = "otel")]
    let otel_resource = {
        let builder = opentelemetry_sdk::Resource::builder().with_service_name(service_name);
        let builder = metadata.version.iter().fold(builder, |builder, version| {
            builder.with_attribute(opentelemetry::KeyValue::new(
                SERVICE_VERSION,
                version.clone(),
            ))
        });
        builder.build()
    };

    // Trace layer
    #[cfg(feature = "otel")]
    let oltp_traces_layer = if let Some(otlp_endpoint) = config
        .tracing
        .otlp
        .as_ref()
        .and_then(|otlp| otlp.trace_endpoint())
    {
        let exporter = match otlp_endpoint {
            crate::config::tracing::OtlpProtocol::Http(endpoint) => SpanExporter::builder()
                .with_http()
                .with_endpoint(endpoint.url.to_string())
                .build()?,
            #[cfg(feature = "otel-grpc")]
            crate::config::tracing::OtlpProtocol::Grpc(endpoint) => SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.url.to_string())
                .build()?,
        };

        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_resource(otel_resource.clone())
            .with_batch_exporter(exporter);

        let provider = config
            .tracing
            .trace_sampling_ratio
            .iter()
            .fold(provider, |provider, ratio| {
                provider.with_sampler(Sampler::TraceIdRatioBased(*ratio))
            });

        let provider = provider.build();
        opentelemetry::global::set_tracer_provider(provider.clone());
        // Create a tracing layer with the configured tracer
        Some(OpenTelemetryLayer::new(
            provider.tracer("tracing-otel-subscriber"),
        ))
    } else {
        None
    };

    // Metric layer
    #[cfg(feature = "otel")]
    let otlp_metrics_layer = if let Some(otlp_endpoint) = config
        .tracing
        .otlp
        .as_ref()
        .and_then(|otlp| otlp.metric_endpoint())
    {
        let exporter = match otlp_endpoint {
            crate::config::tracing::OtlpProtocol::Http(endpoint) => MetricExporter::builder()
                .with_http()
                .with_endpoint(endpoint.url.to_string())
                .build()?,
            #[cfg(feature = "otel-grpc")]
            crate::config::tracing::OtlpProtocol::Grpc(endpoint) => MetricExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint.url.to_string())
                .build()?,
        };
        let reader = PeriodicReader::builder(exporter);
        let reader = if let Some(interval) = config.tracing.metrics_export_interval {
            reader.with_interval(interval)
        } else {
            reader
        };
        let provider = SdkMeterProvider::builder()
            .with_reader(reader.build())
            .with_resource(otel_resource.clone())
            .build();
        opentelemetry::global::set_meter_provider(provider.clone());
        Some(MetricsLayer::new(provider))
    } else {
        None
    };

    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::from_str(&config.tracing.level)?.into())
        .from_env()?;
    let env_filter = config.tracing.trace_filters.iter().try_fold(
        env_filter,
        |env_filter, directive| -> RoadsterResult<EnvFilter> {
            Ok(env_filter.add_directive(directive.parse()?))
        },
    )?;

    let registry = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(compact_log_layer)
        .with(pretty_log_layer)
        .with(json_log_layer);

    #[cfg(feature = "otel")]
    let registry = { registry.with(oltp_traces_layer).with(otlp_metrics_layer) };

    #[cfg_attr(any(test, feature = "testing"), allow(unused_variables))]
    let result = registry.try_init();

    // When running with cargo test, the registry is not reset between tests.
    #[cfg(not(any(test, feature = "testing")))]
    result?;

    Ok(())
}
