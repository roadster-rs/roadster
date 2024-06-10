use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SidekiqError {
    #[error(transparent)]
    Sidekiq(#[from] sidekiq::Error),

    #[error(transparent)]
    Redis(#[from] sidekiq::RedisError),

    #[error(transparent)]
    Bb8(#[from] bb8::RunError<sidekiq::RedisError>),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<sidekiq::Error> for Error {
    fn from(value: sidekiq::Error) -> Self {
        Self::Sidekiq(SidekiqError::from(value))
    }
}

impl From<sidekiq::RedisError> for Error {
    fn from(value: sidekiq::RedisError) -> Self {
        Self::Sidekiq(SidekiqError::from(value))
    }
}

impl From<bb8::RunError<sidekiq::RedisError>> for Error {
    fn from(value: bb8::RunError<sidekiq::RedisError>) -> Self {
        Self::Sidekiq(SidekiqError::from(value))
    }
}
