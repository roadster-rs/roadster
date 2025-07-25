use crate::error::Error;
use std::time::Duration;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum WorkerError {
    #[cfg(feature = "worker-pg")]
    #[error(transparent)]
    PgProcessor(#[from] crate::worker::backend::pg::processor::PgProcessorError),

    #[cfg(feature = "worker-sidekiq")]
    #[error(transparent)]
    SidekiqProcessor(#[from] crate::worker::backend::sidekiq::processor::SidekiqProcessorError),

    #[error(transparent)]
    Enqueue(#[from] EnqueueError),

    #[error(transparent)]
    Dequeue(#[from] DequeueError),

    #[error("An error occurred while handling a job in worker `{0}`: {1}")]
    Handle(String, Box<dyn std::error::Error + Send + Sync>),

    #[error("Worker `{0}` panicked while handling a job")]
    Panic(String),

    #[error("The maximum timeout of `{1:?}` elapsed when handling a job in worker `{0}`")]
    Timeout(String, Duration),

    #[error(transparent)]
    Cron(#[from] cron::error::Error),

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

#[cfg(feature = "worker-sidekiq")]
impl From<crate::worker::backend::sidekiq::processor::SidekiqProcessorError> for Error {
    fn from(value: crate::worker::backend::sidekiq::processor::SidekiqProcessorError) -> Self {
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

impl From<cron::error::Error> for Error {
    fn from(value: cron::error::Error) -> Self {
        Self::Worker(WorkerError::from(value))
    }
}
