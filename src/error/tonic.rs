use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TonicError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Self::Tonic(TonicError::from(value))
    }
}
