use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EmailError {
    #[cfg(feature = "email-smtp")]
    #[error(transparent)]
    Smtp(#[from] lettre::transport::smtp::Error),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(feature = "email-smtp")]
impl From<lettre::transport::smtp::Error> for Error {
    fn from(value: lettre::transport::smtp::Error) -> Self {
        Self::Email(EmailError::from(value))
    }
}
