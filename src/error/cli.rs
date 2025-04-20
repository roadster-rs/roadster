use crate::config::environment::Environment;
use crate::error::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CliError {
    #[error(
        "Running destructive command is not allowed in environment `{0}`. To override, provide the `--allow-dangerous` CLI arg."
    )]
    DestructiveCmdNotAllowed(Environment),

    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
