use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use roadster::app::{PrepareOptions, run_test};
use roadster::service::http::service::HttpService;
use roadster_diesel_example::build_app;
use tower_util::ServiceExt;

#[tokio::test]
async fn health() {
    run_test(build_app(), PrepareOptions::test(), async |app| {
        let http_service = app.service_registry.get::<HttpService>().unwrap();
        let router = http_service.router().clone();

        let request: Request<Body> = Request::builder()
            .uri("/api/_health")
            .body(().into())
            .unwrap();

        let response: Response<Body> = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    })
    .await
    .unwrap()
}
