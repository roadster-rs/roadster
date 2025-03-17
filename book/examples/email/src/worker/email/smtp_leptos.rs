use crate::model::user::User;
use async_trait::async_trait;
use leptos::prelude::*;
use lettre::Transport;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, MessageBuilder};
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use serde::{Deserialize, Serialize};
use sidekiq::Worker;
use std::str::FromStr;
use tracing::{info, instrument};
use typed_builder::TypedBuilder;
use uuid::Uuid;

pub struct EmailConfirmationHtml {
    state: AppContext,
}

impl EmailConfirmationHtml {
    pub fn new(state: &AppContext) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize)]
pub struct EmailConfirmationHtmlArgs {
    user_id: Uuid,
}

#[async_trait]
impl Worker<EmailConfirmationHtmlArgs> for EmailConfirmationHtml {
    #[instrument(skip_all)]
    async fn perform(&self, args: EmailConfirmationHtmlArgs) -> sidekiq::Result<()> {
        let user = User::find_by_id(&self.state, args.user_id).await?;

        send_email(&self.state, &user).await?;

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
