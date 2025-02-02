use aide::transform::TransformOperation;
use axum::extract::State;
use axum::Json;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExampleBResponse {}

#[instrument(skip_all)]
pub async fn example_b_get(
    State(_state): State<AppContext>,
) -> RoadsterResult<Json<ExampleBResponse>> {
    Ok(Json(ExampleBResponse {}))
}

pub fn example_b_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example B API.")
        .tag("Example B")
        .response_with::<200, Json<ExampleBResponse>, _>(|res| res.example(ExampleBResponse {}))
}
