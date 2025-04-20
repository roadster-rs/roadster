use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OtherError {
    #[error(transparent)]
    #[deprecated(
        since = "0.7.3",
        note = "`anyhow` is no longer used internally. This enum variant will be removed in the next semver breaking release."
    )]
    Anyhow(#[from] anyhow::Error),

    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self::Other(OtherError::from(value))
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(value: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Other(OtherError::from(value))
    }
}
