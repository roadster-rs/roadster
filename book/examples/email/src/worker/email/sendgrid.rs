use crate::model::user::User;
use async_trait::async_trait;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use sendgrid::v3::{Email, Message, Personalization};
use serde::{Deserialize, Serialize};
use sidekiq::Worker;
use tracing::{info, instrument};
use typed_builder::TypedBuilder;
use uuid::Uuid;

pub struct EmailConfirmationSendgrid {
    state: AppContext,
}

impl EmailConfirmationSendgrid {
    pub fn new(state: &AppContext) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

#[derive(Debug, TypedBuilder, Serialize, Deserialize)]
pub struct EmailConfirmationSendgridArgs {
    user_id: Uuid,
}

#[async_trait]
impl Worker<EmailConfirmationSendgridArgs> for EmailConfirmationSendgrid {
    #[instrument(skip_all)]
    async fn perform(&self, args: EmailConfirmationSendgridArgs) -> sidekiq::Result<()> {
        let user = User::find_by_id(&self.state, args.user_id).await?;

        send_email(&self.state, &user).await?;

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
