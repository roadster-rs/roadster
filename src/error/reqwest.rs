use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReqwestError {
    #[error(transparent)]
    Error(#[from] reqwest::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(ReqwestError::from(value))
    }
}
