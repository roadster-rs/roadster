use crate::model::user::User;
use async_trait::async_trait;
use leptos::prelude::*;
use lettre::Transport;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, MessageBuilder};
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::worker::Worker;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{info, instrument};
use uuid::Uuid;

#[derive(Debug, bon::Builder, Serialize, Deserialize)]
pub struct EmailConfirmationHtmlArgs {
    user_id: Uuid,
}

pub struct EmailConfirmationHtml;

#[async_trait]
impl Worker<AppContext, EmailConfirmationHtmlArgs> for EmailConfirmationHtml {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::PgEnqueuer;

    #[instrument(skip_all)]
    async fn handle(
        &self,
        state: &AppContext,
        args: EmailConfirmationHtmlArgs,
    ) -> Result<(), Self::Error> {
        let user = User::find_by_id(&state, args.user_id).await?;

        send_email(&state, &user).await?;

        Ok(())
    }
}

/// Send the verification email to the user.
async fn send_email(state: &AppContext, user: &User) -> RoadsterResult<()> {
    let verify_url = "https://exaple.com?verify=1234";

    let body = body(&user.name, verify_url);

    let email: MessageBuilder = (&state.config().email).into();
    let email = email
        .to(Mailbox::from_str(&user.email)?)
        .subject("Please confirm your email address")
        // Set the content type as html
        .header(ContentType::TEXT_HTML)
        .body(body.to_html())?;

    state.smtp().send(&email)?;

    info!(user=%user.id, "Email confirmation sent");
    Ok(())
}

/// Build the email body as HTML using Leptos.
fn body(name: &str, verify_url: &str) -> impl IntoView {
    view! {
        <div>
            <p>"Hello "{name}","</p>
            <p>"Please click the link below to confirm your email address."</p>
            <a href=verify_url rel="noopener noreferrer">
                "Verify email"
            </a>
        </div>
    }
}
