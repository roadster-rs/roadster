use crate::model::user::User;
use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::worker::Worker;
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[derive(Debug, TypedBuilder, Serialize, Deserialize)]
pub struct EmailConfirmationSendgridArgs {
    user_id: Uuid,
}

pub struct EmailConfirmationSendgrid;

#[async_trait]
impl Worker<AppContext, EmailConfirmationSendgridArgs> for EmailConfirmationSendgrid {
    type Error = roadster::error::Error;
    type Enqueuer = roadster::worker::PgEnqueuer;

    #[instrument(skip_all)]
    async fn handle(
        &self,
        state: &AppContext,
        args: EmailConfirmationSendgridArgs,
    ) -> Result<(), Self::Error> {
        let user = User::find_by_id(&state, args.user_id).await?;

        send_email(&state, &user).await?;

        Ok(())
    }
}

const TEMPLATE_ID: &str = "template-id";

#[derive(Serialize)]
struct EmailTemplateArgs {
    verify_url: String,
}

/// Send the verification email to the user.
async fn send_email(state: &AppContext, user: &User) -> RoadsterResult<()> {
    let verify_url = "https://exaple.com?verify=1234".to_string();

    let personalization = Personalization::new(Email::new(&user.email))
        .set_subject("Please confirm your email address")
        .add_dynamic_template_data_json(&EmailTemplateArgs { verify_url })?;

    let message = Message::new(Email::new(state.config().email.from.email.to_string()))
        .set_template_id(TEMPLATE_ID)
        .add_personalization(personalization);

    state.sendgrid().send(&message).await?;

    info!(user=%user.id, "Email confirmation sent");
    Ok(())
}
