use crate::model::user::User;
use async_trait::async_trait;
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

pub struct EmailConfirmationPlainText {
    state: AppContext,
}

impl EmailConfirmationPlainText {
    pub fn new(state: &AppContext) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize)]
pub struct EmailConfirmationPlainTextArgs {
    user_id: Uuid,
}

#[async_trait]
impl Worker<EmailConfirmationPlainTextArgs> for EmailConfirmationPlainText {
    #[instrument(skip_all)]
    async fn perform(&self, args: EmailConfirmationPlainTextArgs) -> sidekiq::Result<()> {
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
        // Set the content type as plaintext
        .header(ContentType::TEXT_PLAIN)
        .body(body)?;

    state.smtp().send(&email)?;

    info!(user=%user.id, "Email confirmation sent");
    Ok(())
}

/// Build the plaintext email content.
fn body(name: &str, verify_url: &str) -> String {
    format!(
        r#"Hello {name},
        
        Please open the below link in your browser to verify your email:
        
        {verify_url}
        "#
    )
}
