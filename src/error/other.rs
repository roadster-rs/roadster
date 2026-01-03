use crate::error::Error;
use std::borrow::Cow;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OtherError {
    #[error("{0}")]
    Message(Cow<'static, str>),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

impl From<Box<dyn Send + Sync + std::error::Error>> for Error {
    fn from(value: Box<dyn Send + Sync + std::error::Error>) -> Self {
        Self::Other(OtherError::from(value))
    }
}
