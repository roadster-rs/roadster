use crate::api::core::health::{HeathCheckResponse, health_check};
use crate::api::http::build_path;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
#[cfg(feature = "open-api")]
use crate::health::check::{CheckResponse, ErrorData, Status};
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
use axum::extract::State;
use axum::extract::{FromRef, Query};
use axum::routing::get;
use axum::{Json, Router};
#[cfg(feature = "open-api")]
use schemars::JsonSchema;
use serde_derive::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;

#[cfg(feature = "open-api")]
const TAG: &str = "Health";

pub fn routes<S>(parent: &str, state: &S) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(state);
    let router = Router::new();
    if !enabled(&context) {
        return router;
    }
    let root = build_path(parent, route(&context));
    router.route(&root, get(health_get::<S>))
}

#[cfg(feature = "open-api")]
pub fn api_routes<S>(parent: &str, state: &S) -> ApiRouter<S>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let context = AppContext::from_ref(state);
    let router = ApiRouter::new();
    if !enabled(&context) {
        return router;
    }
    let root = build_path(parent, route(&context));
    router.api_route(&root, get_with(health_get::<S>, health_get_docs))
}

fn enabled(context: &AppContext) -> bool {
    context
        .config()
        .service
        .http
        .custom
        .default_routes
        .health
        .enabled(context)
}

fn route(context: &AppContext) -> &str {
    &context
        .config()
        .service
        .http
        .custom
        .default_routes
        .health
        .route
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "open-api", derive(JsonSchema))]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct HeathCheckRequest {
    /// Maximum time to spend checking the health of the resources in milliseconds
    ///
    /// Note: If this is greater than the timeout configured in middleware, the request may
    /// time out before the `max_duration` elapses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_duration: Option<u64>,
}

#[instrument(skip_all)]
async fn health_get<S>(
    State(state): State<S>,
    Query(query): Query<HeathCheckRequest>,
) -> RoadsterResult<Json<HeathCheckResponse>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let duration = query
        .max_duration
        .map(Duration::from_millis)
        .unwrap_or_else(|| {
            let context = AppContext::from_ref(&state);
            context.config().health_check.max_duration.api
        });
    Ok(Json(health_check(&state, Some(duration)).await?))
}

#[cfg(feature = "open-api")]
fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse {
                latency: 20,
                resources: std::collections::BTreeMap::from([
                    (
                        "db".to_string(),
                        CheckResponse::builder()
                            .status(Status::Ok)
                            .latency(Duration::from_secs(1))
                            .custom(std::collections::BTreeMap::from([
                                ("foo", 1234),
                                ("bar", 5000),
                            ]))
                            .build(),
                    ),
                    (
                        "redis".to_string(),
                        CheckResponse::builder()
                            .status(Status::Err(
                                ErrorData::builder()
                                    .msg("An error occurred".to_string())
                                    .build(),
                            ))
                            .latency(Duration::from_secs(2))
                            .build(),
                    ),
                ]),
            })
            .description(
                "Health status of the app's resources. Each resource entry will
                contain at least the `status` and `latency` fields, but can also contain arbitrary
                data in the `custom` field.",
            )
        })
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use rstest::rstest;

    // Todo: Is there a better way to structure this test (and the ones in `docs` and `ping`)
    //  to reduce duplication?
    #[rstest]
    #[case(false, None, None, false)]
    #[case(false, Some(false), None, false)]
    #[case(true, None, Some("/foo".to_string()), true)]
    #[case(false, Some(true), None, true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn health(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] route: Option<String>,
        #[case] enabled: bool,
    ) {
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.default_routes.default_enable = default_enable;
        config.service.http.custom.default_routes.health.enable = enable;
        if let Some(route) = route.as_ref() {
            config
                .service
                .http
                .custom
                .default_routes
                .health
                .route
                .clone_from(route);
        }
        let context = AppContext::test(Some(config), None, None).unwrap();

        assert_eq!(super::enabled(&context), enabled);
        assert_eq!(
            super::route(&context),
            route.unwrap_or_else(|| "_health".to_string())
        );
    }
}
