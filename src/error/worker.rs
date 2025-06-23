use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkerError {
    #[cfg(feature = "worker-pg")]
    #[error(transparent)]
    PgProcessor(#[from] crate::worker::backend::pg::processor::PgProcessorError),

    #[error(transparent)]
    Enqueue(#[from] EnqueueError),

    #[error(transparent)]
    Dequeue(#[from] DequeueError),

    #[error("An error occurred while handling a job in worker `{0}`: {1}")]
    Handle(String, Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EnqueueError {
    #[error("No backend configured for worker `{0}`.")]
    NoBackend(String),

    #[error("No queue configured for worker `{0}`.")]
    NoQueue(String),

    #[error("Unable to serialize job args: `{0}`")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DequeueError {
    #[error("Unable to deserialize job args: `{0}`")]
    Serde(#[from] serde_json::Error),
}

#[cfg(feature = "worker-pg")]
impl From<crate::worker::backend::pg::processor::PgProcessorError> for Error {
    fn from(value: crate::worker::backend::pg::processor::PgProcessorError) -> Self {
        Self::Worker(WorkerError::from(value))
    }
}

impl From<EnqueueError> for Error {
    fn from(value: EnqueueError) -> Self {
        Self::Worker(WorkerError::from(value))
    }
}

impl From<DequeueError> for Error {
    fn from(value: DequeueError) -> Self {
        Self::Worker(WorkerError::from(value))
    }
}
