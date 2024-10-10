use crate::config::email::Email;
use crate::config::environment::Environment;
use crate::util::serde::default_true;
use config::{FileFormat, FileSourceString};
use lettre::message::Mailbox;
use reqwest::Client;
use sendgrid::v3::{Message, Sender};
use serde_derive::{Deserialize, Serialize};
use validator::Validate;

pub(crate) fn default_config_per_env(
    environment: Environment,
) -> Option<config::File<FileSourceString, FileFormat>> {
    let config = match environment {
        Environment::Production => Some(include_str!("config/production.toml")),
        _ => None,
    };
    config.map(|c| config::File::from_str(c, FileFormat::Toml))
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Sendgrid {
    /// Your Sendgrid API key.
    pub api_key: String,

    /// Whether messages should be sent in [sandbox mode](https://www.twilio.com/docs/sendgrid/for-developers/sending-email/sandbox-mode).
    ///
    /// Note that this is currently not supported by the [sendgrid crate](https://crates.io/crates/sendgrid).
    #[serde(default = "default_true")]
    pub sandbox: bool,

    /// Whether the Sendgrid client should connect only with https.
    ///
    /// If `true`, the Sendgrid client will only be allowed to connect to the Sendgrid API using
    /// https. If `false`, the Sendgrid client could in theory connect using http.
    ///
    /// This is automatically applied if creating a [`Sender`] using the provided
    /// [`From<&Sendgrid>`] implementation.
    #[serde(default = "default_true")]
    pub https_only: bool,
}

impl From<&Email> for Message {
    fn from(value: &Email) -> Self {
        let message = Message::new(mailbox_to_email(&value.from));
        let message = if let Some(reply_to) = value.reply_to.as_ref() {
            message.set_reply_to(mailbox_to_email(reply_to))
        } else {
            message
        };
        message
    }
}

fn mailbox_to_email(mailbox: &Mailbox) -> sendgrid::v3::Email {
    let email = sendgrid::v3::Email::new(mailbox.email.to_string());
    let email = if let Some(name) = mailbox.name.as_ref() {
        email.set_name(name)
    } else {
        email
    };
    email
}

impl TryFrom<&Sendgrid> for Sender {
    type Error = reqwest::Error;

    fn try_from(value: &Sendgrid) -> Result<Self, Self::Error> {
        let client = Client::builder().https_only(value.https_only).build()?;
        Ok(Sender::new(value.api_key.clone(), Some(client)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::assert_toml_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(
        r#"
        api-key = "api-key"
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        http_only = false
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        http_only = true
        "#
    )]
    #[case(
        r#"
        api-key = "api-key"
        sandbox = false
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let sendgrid: Sendgrid = toml::from_str(config).unwrap();

        assert_toml_snapshot!(sendgrid);
    }
}
