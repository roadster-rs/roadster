use anyhow::anyhow;
use axum::body::{Body, to_bytes};
use axum::http::{Request, Response, StatusCode};
use full::app::App;
use roadster::app::{PrepareOptions, run_test_with_result};
use roadster::error::RoadsterResult;
use roadster::service::http::service::HttpService;
use tower_util::ServiceExt;

#[tokio::test]
async fn health() {
    run_test_with_result(
        App,
        PrepareOptions::test(),
        async |app| -> RoadsterResult<()> {
            let response = app
                .service_registry
                .invoke(
                    async |srvc: &HttpService| -> RoadsterResult<Response<Body>> {
                        let router = srvc.router().clone();
                        let request: Request<Body> =
                            Request::builder().uri("/api/_health").body(().into())?;

                        Ok(router.oneshot(request).await?)
                    },
                )
                .await??;

            if response.status() != StatusCode::OK {
                return Err(anyhow!(
                    "Health checks failed: {:?}",
                    to_bytes(response.into_body(), usize::MAX).await
                )
                .into());
            }

            Ok(())
        },
    )
    .await
    .unwrap()
}
