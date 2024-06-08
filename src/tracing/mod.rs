use std::str::FromStr;

use crate::app::metadata::AppMetadata;
#[cfg(feature = "otel")]
use convert_case::{Case, Casing};
#[cfg(feature = "otel")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "otel")]
use opentelemetry_sdk::metrics::reader::{DefaultAggregationSelector, DefaultTemporalitySelector};
#[cfg(feature = "otel")]
use opentelemetry_sdk::propagation::TraceContextPropagator;
#[cfg(feature = "otel")]
use opentelemetry_sdk::runtime::Tokio;
#[cfg(feature = "otel")]
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::Level;
#[cfg(feature = "otel")]
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::config::app_config::AppConfig;
use crate::error::RoadsterResult;

// Todo: make this configurable
pub fn init_tracing(
    config: &AppConfig,
    #[allow(unused_variables)] // This parameter isn't used in some feature combinations
    metadata: &AppMetadata,
) -> RoadsterResult<()> {
    // Stdout Layer
    let stdout_layer = tracing_subscriber::fmt::layer();

    #[cfg(feature = "otel")]
    if config.tracing.trace_propagation {
        opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
    }

    #[cfg(feature = "otel")]
    let otel_resource = {
        let service_name = config
            .tracing
            .service_name
            .clone()
            .or(metadata.name.clone())
            .unwrap_or(config.app.name.to_case(Case::Snake));

        let mut resource_metadata = vec![opentelemetry::KeyValue::new(SERVICE_NAME, service_name)];

        if let Some(version) = metadata.version.clone() {
            resource_metadata.push(opentelemetry::KeyValue::new(SERVICE_VERSION, version))
        }

        opentelemetry_sdk::Resource::new(resource_metadata)
    };

    // Trace layer
    #[cfg(feature = "otel")]
    let oltp_traces_layer = if let Some(otlp_endpoint) = config.tracing.otlp_endpoint.as_ref() {
        let otlp_tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(otlp_endpoint.to_string()),
            )
            .with_trace_config(
                opentelemetry_sdk::trace::config().with_resource(otel_resource.clone()),
            )
            .install_batch(Tokio)?;
        // Create a tracing layer with the configured tracer
        Some(tracing_opentelemetry::layer().with_tracer(otlp_tracer))
    } else {
        None
    };

    // Metric layer
    #[cfg(feature = "otel")]
    let otlp_metrics_layer = if let Some(otlp_endpoint) = config.tracing.otlp_endpoint.as_ref() {
        let provider = opentelemetry_otlp::new_pipeline()
            .metrics(Tokio)
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(otlp_endpoint.clone()),
            )
            .with_resource(otel_resource)
            .with_aggregation_selector(DefaultAggregationSelector::new())
            .with_temporality_selector(DefaultTemporalitySelector::new())
            .build()?;
        opentelemetry::global::set_meter_provider(provider.clone());
        Some(MetricsLayer::new(provider))
    } else {
        None
    };

    // Hide some noisy logs from traces
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::from_str(&config.tracing.level)?.into())
        .from_env()?
        .add_directive("h2=warn".parse()?)
        .add_directive("tower::buffer::worker=warn".parse()?);

    let registry = tracing_subscriber::Registry::default()
        .with(env_filter)
        .with(stdout_layer);

    #[cfg(feature = "otel")]
    let registry = { registry.with(oltp_traces_layer).with(otlp_metrics_layer) };

    registry.try_init()?;

    Ok(())
}
