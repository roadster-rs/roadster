use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<config::ConfigError> for Error {
    fn from(value: config::ConfigError) -> Self {
        Self::Config(ConfigError::from(value))
    }
}
