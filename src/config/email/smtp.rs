use crate::config::email::Email;
use lettre::message::MessageBuilder;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::SmtpTransportBuilder;
use lettre::SmtpTransport;
use serde_derive::{Deserialize, Serialize};
use url::Url;
use validator::{Validate, ValidationErrors};

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Smtp {
    #[validate(nested)]
    pub connection: SmtpConnection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged, rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SmtpConnection {
    Fields(SmtpConnectionFields),
    Uri(SmtpConnectionUri),
}

impl Validate for SmtpConnection {
    fn validate(&self) -> Result<(), ValidationErrors> {
        match self {
            SmtpConnection::Fields(fields) => fields.validate(),
            SmtpConnection::Uri(uri) => uri.validate(),
        }
    }
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SmtpConnectionFields {
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SmtpConnectionUri {
    pub uri: Url,
}

impl From<&Email> for MessageBuilder {
    fn from(value: &Email) -> Self {
        let builder = MessageBuilder::new().from(value.from.clone());
        let builder = if let Some(reply_to) = value.reply_to.as_ref() {
            builder.reply_to(reply_to.clone())
        } else {
            builder
        };

        builder
    }
}

impl TryFrom<&SmtpConnection> for SmtpTransportBuilder {
    type Error = lettre::transport::smtp::Error;

    fn try_from(value: &SmtpConnection) -> Result<Self, Self::Error> {
        match value {
            SmtpConnection::Fields(fields) => {
                let credentials =
                    Credentials::new(fields.username.clone(), fields.password.clone());
                SmtpTransport::relay(&fields.host)
                    .map(|builder| {
                        if let Some(port) = fields.port {
                            builder.port(port)
                        } else {
                            builder
                        }
                    })
                    .map(|builder| builder.credentials(credentials))
            }
            SmtpConnection::Uri(fields) => SmtpTransport::from_url(fields.uri.as_ref()),
        }
    }
}

impl TryFrom<&SmtpConnection> for SmtpTransport {
    type Error = lettre::transport::smtp::Error;

    fn try_from(value: &SmtpConnection) -> Result<Self, Self::Error> {
        let builder: SmtpTransportBuilder = value.try_into()?;
        Ok(builder.build())
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
        [from]
        email = "no-reply@example.com"

        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"
        "#
    )]
    #[case(
        r#"
        from = "No Reply <no-reply@example.com>"

        [reply-to]
        email = "no-reply@example.com"
        name = "No Reply"

        [smtp.connection]
        host = "smtp.example.com"
        username = "username"
        password = "password"
        "#
    )]
    #[case(
        r#"
        [from]
        email = "no-reply@example.com"

        [smtp.connection]
        host = "smtp.example.com"
        username = "username"
        password = "password"
        "#
    )]
    #[case(
        r#"
        reply-to = "No Reply <no-reply@example.com>"

        [from]
        email = "no-reply@example.com"
        name = "No Reply"

        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"
        "#
    )]
    #[case(
        r#"
        from = "no-reply@example.com"

        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"
        "#
    )]
    #[case(
        r#"
        from = "no-reply@example.com"

        [smtp.connection]
        host = "smtp.example.com"
        port = 465
        username = "username"
        password = "password"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let email: Email = toml::from_str(config).unwrap();

        assert_toml_snapshot!(email);
    }
}
