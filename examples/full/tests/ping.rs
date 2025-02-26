use axum::body::Body;
use axum::http::{Request, StatusCode};
use full::app::App;
use roadster::app::{PrepareOptions, run_test};
use roadster::service::http::service::HttpService;
use tower_util::ServiceExt;

#[tokio::test]
async fn ping() {
    run_test(App, PrepareOptions::test(), async |app| {
        let http_service = app.service_registry.get::<HttpService>().unwrap();
        let router = http_service.router().clone();

        let request: Request<Body> = Request::builder()
            .uri("/api/_ping")
            .body(().into())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    })
    .await
    .unwrap()
}
