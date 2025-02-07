use crate::app_state::AppState;
use crate::worker::example::ExampleWorker;
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::Json;
use lettre::message::header::ContentType;
use lettre::message::{Mailbox, MessageBuilder};
use lettre::Transport;
use roadster::api::http::build_path;
use roadster::error::RoadsterResult;
use roadster::service::worker::sidekiq::app_worker::AppWorker;
use schemars::JsonSchema;
use sendgrid::v3::{Content, Email, Personalization};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{instrument, warn};

const BASE: &str = "/example";
const TAG: &str = "Example";

pub fn routes(parent: &str) -> ApiRouter<AppState> {
    let root = build_path(parent, BASE);

    ApiRouter::new().api_route(&root, get_with(example_get, example_get_docs))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExampleResponse {}

#[instrument(skip_all)]
async fn example_get(State(state): State<AppState>) -> RoadsterResult<Json<ExampleResponse>> {
    ExampleWorker::enqueue(&state, "Example".to_string()).await?;

    // Emails can be sent via SMTP
    let email: MessageBuilder = (&state.app_context.config().email).into();
    let email = email
        .to(Mailbox::from_str("hello@example.com")?)
        .subject("Greetings")
        .header(ContentType::TEXT_PLAIN)
        .body("Hello, World!".to_string())?;
    state.app_context.smtp().send(&email)?;

    // Emails can also be sent using Sendgrid
    let email: sendgrid::v3::Message = (&state.app_context.config().email).into();
    let email = email
        .set_subject("Greetings")
        .add_content(
            Content::new()
                .set_content_type("text/plain")
                .set_value("Hello, World!"),
        )
        .add_personalization(Personalization::new(Email::new("hello@example.com")));
    if let Err(err) = state.app_context.sendgrid().send(&email).await {
        warn!("An error occurred when sending email using Sendgrid. This may be expected in a dev/test environment if a prod API key is not used. Error: {err}");
    }

    Ok(Json(ExampleResponse {}))
}

fn example_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example API.")
        .tag(TAG)
        .response_with::<200, Json<ExampleResponse>, _>(|res| res.example(ExampleResponse {}))
}
