#[cfg(feature = "email-sendgrid")]
pub mod sendgrid;
#[cfg(feature = "email-smtp")]
pub mod smtp;

use lettre::message::Mailbox;
#[cfg(feature = "email-sendgrid")]
use sendgrid::Sendgrid;
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
    #[cfg(feature = "email-sendgrid")]
    #[validate(nested)]
    pub sendgrid: Sendgrid,
}

#[cfg(all(
    test,
    feature = "email",
    feature = "email-smtp",
    feature = "email-sendgrid"
))]
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

        [sendgrid]
        api-key = "api-key"
        "#
    )]
    #[case(
        r#"
        from = "No Reply <no-reply@example.com>"

        [reply-to]
        email = "no-reply@example.com"
        name = "No Reply"

        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"

        [sendgrid]
        api-key = "api-key"
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

        [sendgrid]
        api-key = "api-key"
        "#
    )]
    #[case(
        r#"
        from = "no-reply@example.com"

        [smtp.connection]
        uri = "smtps://username:password@smtp.example.com:425"

        [sendgrid]
        api-key = "api-key"
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn serialization(_case: TestCase, #[case] config: &str) {
        let email: Email = toml::from_str(config).unwrap();

        assert_toml_snapshot!(email);
    }
}
