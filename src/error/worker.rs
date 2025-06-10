use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkerError {
    #[error(transparent)]
    Enqueue(#[from] EnqueueError),

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
}

impl From<EnqueueError> for Error {
    fn from(value: EnqueueError) -> Self {
        Self::Worker(WorkerError::from(value))
    }
}
