use axum::body::Body;
use axum::http::{Request, StatusCode};
use full::app::App;
use roadster::app::{PrepareOptions, run_test};
use roadster::service::http::service::HttpService;
use tower_util::ServiceExt;

#[tokio::test]
async fn ping() {
    run_test(App, PrepareOptions::test(), async |app| {
        let response = app
            .service_registry
            .invoke(async |srvc: &HttpService| {
                let router = srvc.router().clone();

                let request: Request<Body> = Request::builder()
                    .uri("/api/_ping")
                    .body(().into())
                    .unwrap();

                router.oneshot(request).await.unwrap()
            })
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    })
    .await
    .unwrap()
}
