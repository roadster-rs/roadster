use crate::config::email::Email;
use lettre::SmtpTransport;
use lettre::message::MessageBuilder;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::{PoolConfig, SmtpTransportBuilder};
use serde_derive::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Duration;
use url::Url;
use validator::{Validate, ValidationErrors};

#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct Smtp {
    #[validate(nested)]
    pub connection: SmtpConnection,

    #[serde(default)]
    #[validate(nested)]
    pub pool: Option<SmtpPool>,
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

#[serde_as]
#[derive(Debug, Default, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct SmtpPool {
    #[serde(default)]
    pub min_connections: Option<u32>,

    #[serde(default)]
    pub max_connections: Option<u32>,

    #[serde(default)]
    #[serde_as(as = "Option<serde_with::DurationSeconds>")]
    pub idle_timeout: Option<Duration>,
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

impl TryFrom<&Smtp> for SmtpTransportBuilder {
    type Error = lettre::transport::smtp::Error;

    fn try_from(value: &Smtp) -> Result<Self, Self::Error> {
        let builder = match &value.connection {
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
        }?;

        let builder = if let Some(smtp_pool) = value.pool.as_ref() {
            builder.pool_config(smtp_pool.into())
        } else {
            builder
        };

        Ok(builder)
    }
}

impl From<&SmtpPool> for PoolConfig {
    fn from(value: &SmtpPool) -> Self {
        let pool_config = PoolConfig::new();

        let pool_config = if let Some(min_connections) = value.min_connections {
            pool_config.min_idle(min_connections)
        } else {
            pool_config
        };

        let pool_config = if let Some(max_connections) = value.max_connections {
            pool_config.max_size(max_connections)
        } else {
            pool_config
        };

        if let Some(idle_timeout) = value.idle_timeout {
            pool_config.idle_timeout(idle_timeout)
        } else {
            pool_config
        }
    }
}

impl TryFrom<&Smtp> for SmtpTransport {
    type Error = lettre::transport::smtp::Error;

    fn try_from(value: &Smtp) -> Result<Self, Self::Error> {
        let builder: SmtpTransportBuilder = value.try_into()?;
        Ok(builder.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::snapshot::TestCase;
    use insta::{assert_debug_snapshot, assert_toml_snapshot};
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(
        r#"
        [connection]
        uri = "smtps://username:password@smtp.example.com:425"
        "#
    )]
    #[case(
        r#"
        [connection]
        host = "smtp.example.com"
        username = "username"
        password = "password"
        "#
    )]
    #[case(
        r#"
        [connection]
        host = "smtp.example.com"
        port = 465
        username = "username"
        password = "password"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let smtp: Smtp = toml::from_str(config).unwrap();
        smtp.validate().unwrap();

        assert_toml_snapshot!(smtp);
    }

    #[rstest]
    #[case(
        r#"
        from = "foo@example.com"
        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"
        [sendgrid]
        api-key = "api-key"
        "#
    )]
    #[case(
        r#"
        from = "foo@example.com"
        reply-to = "no-reply@example.com"
        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"
        [sendgrid]
        api-key = "api-key"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn message_builder_from_email(_case: TestCase, #[case] email_config: &str) {
        let email: Email = toml::from_str(email_config).unwrap();
        let msg_builder: MessageBuilder = (&email).into();
        assert_debug_snapshot!(msg_builder);
    }

    #[rstest]
    #[case(
        r#"
        [connection]
        uri = "smtps://username:password@smtp.example.com:425"
        "#
    )]
    #[case(
        r#"
        [connection]
        host = "smtp.example.com"
        port = 465
        username = "username"
        password = "password"
        "#
    )]
    #[case(
        r#"
        [connection]
        host = "smtp.example.com"
        username = "username"
        password = "password"
        "#
    )]
    #[case(
        r#"
        [connection]
        uri = "smtps://username:password@smtp.example.com:425"
        [pool]
        min-connections = 1
        "#
    )]
    #[case(
        r#"
        [connection]
        uri = "smtps://username:password@smtp.example.com:425"
        [pool]
        max-connections = 100
        "#
    )]
    #[case(
        r#"
        [connection]
        uri = "smtps://username:password@smtp.example.com:425"
        [pool]
        idle-timeout = 60
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn smtp_connection_from_config(_case: TestCase, #[case] smtp_config: &str) {
        let smtp: Smtp = toml::from_str(smtp_config).unwrap();
        let _smtp_transport: SmtpTransport = (&smtp).try_into().unwrap();
    }

    #[rstest]
    #[case(
        r#"
        [connection]
        uri = "https://username:password@smtp.example.com:425"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn smtp_connection_from_config_err(_case: TestCase, #[case] smtp_config: &str) {
        let smtp: Smtp = toml::from_str(smtp_config).unwrap();
        assert!(SmtpTransport::try_from(&smtp).is_err());
    }
}
