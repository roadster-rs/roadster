use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TokioError {
    #[error(transparent)]
    Timeout(#[from] tokio::time::error::Elapsed),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

impl From<tokio::time::error::Elapsed> for Error {
    fn from(value: tokio::time::error::Elapsed) -> Self {
        Self::Tokio(TokioError::from(value))
    }
}
