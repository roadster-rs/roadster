use crate::api::core::health::{health_check, HeathCheckResponse};
#[cfg(any(feature = "db-sql", feature = "sidekiq"))]
use crate::api::core::health::{ResourceHealth, Status};
use crate::api::http::build_path;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
#[cfg(feature = "open-api")]
use aide::axum::routing::get_with;
#[cfg(feature = "open-api")]
use aide::axum::ApiRouter;
#[cfg(feature = "open-api")]
use aide::transform::TransformOperation;
use axum::extract::FromRef;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
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

#[instrument(skip_all)]
async fn health_get<S>(State(state): State<S>) -> RoadsterResult<Json<HeathCheckResponse>>
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    let health = health_check::<S>(&state).await?;
    Ok(Json(health))
}

#[cfg(feature = "open-api")]
fn health_get_docs(op: TransformOperation) -> TransformOperation {
    op.description("Check the health of the server and its resources.")
        .tag(TAG)
        .response_with::<200, Json<HeathCheckResponse>, _>(|res| {
            res.example(HeathCheckResponse {
                latency: 20,
                #[cfg(feature = "db-sql")]
                db: ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: None,
                    ping_latency: None,
                    latency: 10,
                },
                #[cfg(feature = "sidekiq")]
                redis_enqueue: ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: Some(5),
                    ping_latency: Some(10),
                    latency: 15,
                },
                #[cfg(feature = "sidekiq")]
                redis_fetch: Some(ResourceHealth {
                    status: Status::Ok,
                    acquire_conn_latency: Some(15),
                    ping_latency: Some(20),
                    latency: 35,
                }),
            })
        })
}

#[cfg(test)]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::app_config::AppConfig;
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
