use crate::error::Error;

#[derive(Debug, Error)]
pub enum TracingError {
    /// An error that occurs during tracing initialization.
    #[error(transparent)]
    Init(#[from] TracingInitError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Error)]
pub enum TracingInitError {
    #[cfg(feature = "otel")]
    #[error(transparent)]
    OtelTrace(#[from] opentelemetry::trace::TraceError),

    #[cfg(feature = "otel")]
    #[error(transparent)]
    OtelMetrics(#[from] opentelemetry::metrics::MetricsError),

    #[error(transparent)]
    ParseLevel(#[from] tracing::metadata::ParseLevelError),

    #[error(transparent)]
    ParseFilter(#[from] tracing_subscriber::filter::ParseError),

    #[error(transparent)]
    FilterFromEnv(#[from] tracing_subscriber::filter::FromEnvError),

    #[error(transparent)]
    Init(#[from] tracing_subscriber::util::TryInitError),
}

#[cfg(feature = "otel")]
impl From<opentelemetry::trace::TraceError> for Error {
    fn from(value: opentelemetry::trace::TraceError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

#[cfg(feature = "otel")]
impl From<opentelemetry::metrics::MetricsError> for Error {
    fn from(value: opentelemetry::metrics::MetricsError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

impl From<tracing::metadata::ParseLevelError> for Error {
    fn from(value: tracing::metadata::ParseLevelError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

impl From<tracing_subscriber::filter::ParseError> for Error {
    fn from(value: tracing_subscriber::filter::ParseError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

impl From<tracing_subscriber::filter::FromEnvError> for Error {
    fn from(value: tracing_subscriber::filter::FromEnvError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

impl From<tracing_subscriber::util::TryInitError> for Error {
    fn from(value: tracing_subscriber::util::TryInitError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}
