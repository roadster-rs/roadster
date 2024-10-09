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
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::instrument;

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

    let email: MessageBuilder = (&state.app_context.config().email).into();
    let email = email
        .to(Mailbox::from_str("hello@example.com")?)
        .header(ContentType::TEXT_PLAIN)
        .body("Hello, World!".to_string())?;
    state.app_context.mailer().send(&email)?;

    Ok(Json(ExampleResponse {}))
}

fn example_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example API.")
        .tag(TAG)
        .response_with::<200, Json<ExampleResponse>, _>(|res| res.example(ExampleResponse {}))
}
