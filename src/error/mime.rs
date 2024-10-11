use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MimeError {
    #[error(transparent)]
    FromStr(#[from] mime::FromStrError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<mime::FromStrError> for Error {
    fn from(value: mime::FromStrError) -> Self {
        Self::Mime(MimeError::from(value))
    }
}
