use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TonicError {
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),

    #[error(transparent)]
    ServerReflection(#[from] tonic_reflection::server::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Self::Tonic(TonicError::from(value))
    }
}

impl From<tonic_reflection::server::Error> for Error {
    fn from(value: tonic_reflection::server::Error) -> Self {
        Self::Tonic(TonicError::from(value))
    }
}
