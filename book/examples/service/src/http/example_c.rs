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
pub struct ExampleCResponse {}

#[instrument(skip_all)]
pub async fn example_c_get(
    State(_state): State<AppContext>,
) -> RoadsterResult<Json<ExampleCResponse>> {
    Ok(Json(ExampleCResponse {}))
}

pub fn example_c_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example C API.")
        .tag("Example C")
        .response_with::<200, Json<ExampleCResponse>, _>(|res| res.example(ExampleCResponse {}))
}
