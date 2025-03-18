# Sendgrid

Sendgrid is a popular provider for sending email with a visual email template builder. When the `email-sendgrid` feature
is enabled, Roadster will initialize a Sendgrid client via the [`sendgrid`](https://docs.rs/sendgrid/0.23.0/sendgrid/)
crate.

Sendgrid also supports sending emails via SMTP. For details on Roadster's SMTP integration,
see [the previous chapter](./smtp.md).

## Configure the Sendgrid integration

The Sendgrid connection details can be configured via your app's config files, and via env vars or an [
`AsyncSource`](/features/configuration.html#custom-async-sources) for
sensitive connection details.

```toml
{{ #include ../../../examples/email/config/development/sendgrid.toml}}
```

## Sending an email

With the Sendgrid client, emails are sent by providing a Sendgrid email template ID and the template's parameters. Below
is a simple example of using the Sendgrid client.
See [Sendgrid's docs](https://www.twilio.com/docs/sendgrid/api-reference) for more details on the other fields you may
want to set when sending emails.

```rust,ignore
{{#include ../../../examples/email/src/worker/email/sendgrid.rs:12:}}
```

## Docs.rs links

- [email config](https://docs.rs/roadster/latest/roadster/config/email/struct.Email.html)
