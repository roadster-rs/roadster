use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PgmqError {
    #[error(transparent)]
    Pgmq(#[from] pgmq::PgmqError),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

impl From<pgmq::PgmqError> for Error {
    fn from(value: pgmq::PgmqError) -> Self {
        Self::Pgmq(PgmqError::from(value))
    }
}
