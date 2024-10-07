#[cfg(feature = "email-smtp")]
pub mod smtp;

use lettre::message::Mailbox;
use serde_derive::{Deserialize, Serialize};
#[cfg(feature = "email-smtp")]
use smtp::Smtp;
use validator::Validate;

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Email {
    pub from: Mailbox,
    pub reply_to: Option<Mailbox>,
    #[cfg(feature = "email-smtp")]
    #[validate(nested)]
    pub smtp: Smtp,
}
