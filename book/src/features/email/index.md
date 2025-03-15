# Email

Sending emails is a major requirement of many web apps. At a minimum, a web app will almost certainly want to send
emails related to auth, e.g. account verification and password recovery.

Roadster provides some minimal configuration of email clients, either via plain SMTP with
the [lettre](https://docs.rs/lettre/latest/lettre/) crate, or via Sendgrid with
the [sendgrid](https://docs.rs/sendgrid/latest/sendgrid/) crate.

In either case, emails should generally be sent in a background process, e.g. via a Sidekiq worker.

In the future, we may provide some utilities to reduce the boilerplate required to send an email, e.g. by providing some
`EmailWorker` trait/struct that provides common functionality for all emails. This is not implemented yet, but it
is something we'd like to add in the future.

The following chapters cover the provided SMTP and Sendgrid integrations.

## Docs.rs links

- [`email` mod](https://docs.rs/roadster/latest/roadster/config/email/index.html)
