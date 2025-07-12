use crate::model::user::User;
use async_trait::async_trait;
use lettre::Transport;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, MessageBuilder};
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::worker::Worker;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{info, instrument};
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[derive(Debug, TypedBuilder, Serialize, Deserialize)]
pub struct EmailConfirmationPlainTextArgs {
    user_id: Uuid,
}

pub struct EmailConfirmationPlainText;

#[async_trait]
impl Worker<AppContext, EmailConfirmationPlainTextArgs> for EmailConfirmationPlainText {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::PgEnqueuer;

    #[instrument(skip_all)]
    async fn handle(
        &self,
        state: &AppContext,
        args: EmailConfirmationPlainTextArgs,
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
