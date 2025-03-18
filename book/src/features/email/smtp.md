# SMTP

Roadster's SMTP support allows sending emails with virtually any email provider. The caveat with using plain SMTP is
you may not be able to use the visual email builders provided by some vendors (e.g. Sendgrid or Customer.io). This
means that while you're code will be decoupled from any particular vendor, some additional work will be needed to send
"pretty" emails that match your app's design style. However, if you just need to send plain text emails, or are willing
to do the additional work to create "pretty" emails, SMTP is a great option for sending emails for your app.

## Starting a local SMTP service for testing

There are several SMTP servers that can be run locally for testing. This is another benefit of using SMTP instead of
Sendgrid -- a local SMTP instance can be used to easily verify the contents of your emails and your sending logic, while
Sendgrid only provides minimal dev/testing support and can't be run locally.

Below are a few options for development SMTP services that can be easily run locally with docker.

### [Mailpit](https://github.com/axllent/mailpit)

```shell
docker run -d -p 8025:8025 -p 1025:1025 axllent/mailpit
```

### [smtp4dev](https://github.com/rnwood/smtp4dev)

```shell
docker run -d -p 1080:80 -p 1025:25 rnwood/smtp4dev
```

### [maildev](https://github.com/maildev/maildev)

```shell
docker run -d -p 1080:1080 -p 1025:1025 maildev/maildev
```

## Configure the SMTP integration

The SMTP connection details can be configured via your app's config files, and via env vars or an [
`AsyncSource`](/features/configuration.html#custom-async-sources) for
sensitive connection details. Below is a sample config file that can be used to connect to a locally-hosted SMTP
service.

```toml
{{ #include ../../../examples/email/config/development/smtp.toml}}
```

## Sending plaintext emails

The easiest way to send an email is to simply send a plaintext email.

```rust,ignore
{{#include ../../../examples/email/src/worker/email/smtp.rs:15:}}
```

## Sending html emails with Leptos

In order to send HTML content via SMTP, you can either manually write your HTML, or use something
like [Leptos](https://docs.rs/leptos/latest/leptos/). Leptos is a reactive Rust UI framework, but it can also be used as
a simple HTML templating system. It may be possible to use other frameworks such
as [Yew](https://docs.rs/yew/latest/yew/) or [Dioxus](https://docs.rs/dioxus/latest/dioxus/) as well.

The below example is the same as the plaintext example, except it formats the email message with HTML using Leptos.

```rust,ignore
{{#include ../../../examples/email/src/worker/email/smtp_leptos.rs:16:}}
```

## Docs.rs links

- [email config](https://docs.rs/roadster/latest/roadster/config/email/struct.Email.html)