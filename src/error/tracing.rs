use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TracingError {
    /// An error that occurs during tracing initialization.
    #[error(transparent)]
    Init(#[from] TracingInitError),

    #[error(transparent)]
    #[cfg(any(feature = "worker-pg", feature = "db-sea-orm"))]
    ParseLevel(#[from] log::ParseLevelError),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TracingInitError {
    #[cfg(feature = "otel")]
    #[error(transparent)]
    OtelTrace(#[from] opentelemetry_sdk::trace::TraceError),

    #[cfg(feature = "otel")]
    #[error(transparent)]
    ExporterBuilder(#[from] opentelemetry_otlp::ExporterBuildError),

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
impl From<opentelemetry_sdk::trace::TraceError> for Error {
    fn from(value: opentelemetry_sdk::trace::TraceError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

#[cfg(feature = "otel")]
impl From<opentelemetry_otlp::ExporterBuildError> for Error {
    fn from(value: opentelemetry_otlp::ExporterBuildError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

impl From<tracing::metadata::ParseLevelError> for Error {
    fn from(value: tracing::metadata::ParseLevelError) -> Self {
        Self::Tracing(TracingError::from(TracingInitError::from(value)))
    }
}

#[cfg(any(feature = "worker-pg", feature = "db-sea-orm"))]
impl From<log::ParseLevelError> for Error {
    fn from(value: log::ParseLevelError) -> Self {
        Self::Tracing(TracingError::from(value))
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
