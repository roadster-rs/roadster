use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub(crate) fn default_config() -> config::File<FileSourceString, FileFormat> {
    config::File::from_str(include_str!("default.toml"), FileFormat::Toml)
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Testing {
    #[serde(default = "default_true")]
    pub catch_panic: bool,
}
