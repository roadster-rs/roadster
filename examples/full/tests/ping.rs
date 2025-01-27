use axum::body::Body;
use axum::http::{Request, StatusCode};
use full::app::App;
use roadster::app::{prepare, PrepareOptions};
use roadster::config::environment::Environment;
use roadster::service::http::service::HttpService;
use tower_util::ServiceExt;

#[tokio::test]
async fn ping() {
    let prepared_app = prepare(
        App,
        PrepareOptions::builder().env(Environment::Test).build(),
    )
    .await
    .unwrap();
    let http_service = prepared_app.service_registry.get::<HttpService>().unwrap();
    let router = http_service.router().clone();

    let request: Request<Body> = Request::builder()
        .uri("/api/_ping")
        .body(().into())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
