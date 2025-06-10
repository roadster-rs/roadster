use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkerError {
    #[error(transparent)]
    Enqueue(#[from] crate::service::worker::EnqueueError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<crate::service::worker::EnqueueError> for Error {
    fn from(value: crate::service::worker::EnqueueError) -> Self {
        Self::Worker(WorkerError::from(value))
    }
}
