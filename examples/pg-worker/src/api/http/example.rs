use crate::worker::example::{ExampleWorker, ExampleWorkerArgs};
use aide::axum::ApiRouter;
use aide::axum::routing::get_with;
use aide::transform::TransformOperation;
use axum::Json;
use axum::extract::State;
use roadster::api::http::build_path;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::worker::Worker;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

const BASE: &str = "/example";
const TAG: &str = "Example";

pub fn routes(parent: &str) -> ApiRouter<AppContext> {
    let root = build_path(parent, BASE);

    ApiRouter::new().api_route(&root, get_with(example_get, example_get_docs))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExampleResponse {}

#[instrument(skip_all)]
async fn example_get(State(state): State<AppContext>) -> RoadsterResult<Json<ExampleResponse>> {
    ExampleWorker::enqueue(
        &state,
        ExampleWorkerArgs::builder().foo("foo").bar(1234).build(),
    )
    .await?;

    Ok(Json(ExampleResponse {}))
}

fn example_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example API.")
        .tag(TAG)
        .response_with::<200, Json<ExampleResponse>, _>(|res| res.example(ExampleResponse {}))
}
