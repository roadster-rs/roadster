use crate::app_state::AppState;
use crate::models::user::NewUser;
use aide::axum::routing::get_with;
use aide::axum::ApiRouter;
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::Json;
use diesel::SelectableHelper;
use diesel_async::RunQueryDsl;
use roadster::api::http::build_path;
use roadster::error::RoadsterResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::instrument;

const BASE: &str = "/example";
const TAG: &str = "Example";

pub fn routes(parent: &str) -> ApiRouter<AppState> {
    let root = build_path(parent, BASE);

    ApiRouter::new().api_route(&root, get_with(example_get, example_get_docs))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExampleResponse {
    name: String,
    username: String,
    email: String,
}

#[instrument(skip_all)]
async fn example_get(State(state): State<AppState>) -> RoadsterResult<Json<ExampleResponse>> {
    use fake::faker::internet::raw::{Password, SafeEmail, Username};
    use fake::faker::name::raw::*;
    use fake::locales::*;
    use fake::Fake;

    let name: String = Name(EN).fake();
    let username: String = Username(EN).fake();
    let email: String = SafeEmail(EN).fake();
    let password: String = Password(EN, 10..200).fake();

    let user = NewUser::new(&name, &username, &email, &password);

    let mut conn = state.db().get().await?;

    let user = diesel::insert_into(crate::schema::user::table)
        .values(&user)
        .returning(crate::models::user::User::as_returning())
        .get_result(&mut conn)
        .await?;

    Ok(Json(ExampleResponse {
        name: user.name,
        username: user.username,
        email: user.email,
    }))
}

fn example_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Example API.")
        .tag(TAG)
        .response_with::<200, Json<ExampleResponse>, _>(|res| {
            use fake::faker::internet::raw::{SafeEmail, Username};
            use fake::faker::name::raw::*;
            use fake::locales::*;
            use fake::Fake;

            let name: String = Name(EN).fake();
            let username: String = Username(EN).fake();
            let email: String = SafeEmail(EN).fake();

            res.example(ExampleResponse {
                name,
                username,
                email,
            })
        })
}
