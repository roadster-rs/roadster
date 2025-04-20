use crate::app::App;
use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::AppService;
use async_trait::async_trait;
use axum_core::extract::FromRef;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tonic::transport::server::Router;
use tracing::info;

/// Simple wrapper around a tonic [Router] to run a gRPC service.
// todo: enable adding middleware to the service?
// todo: enable sharing middleware with the axum router?
pub struct GrpcService {
    pub(crate) router: Mutex<Router>,
}

impl GrpcService {
    pub fn new(router: Router) -> Self {
        Self {
            router: Mutex::new(router),
        }
    }
}

#[async_trait]
impl<A, S> AppService<A, S> for GrpcService
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
    A: App<S> + 'static,
{
    fn name(&self) -> String {
        "grpc".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        let context = AppContext::from_ref(state);
        context.config().service.grpc.common.enabled(&context)
    }

    async fn run(
        self: Box<Self>,
        state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        let context = AppContext::from_ref(state);
        let server_addr = context.config().service.grpc.custom.address.url();
        info!("gRPC server will start at {server_addr}");

        self.router
            .into_inner()
            .map_err(|e| {
                crate::error::other::OtherError::Message(format!(
                    "Unable to start GrpcService, mutex was poisoned: {e}"
                ))
            })?
            .serve_with_shutdown(
                server_addr.parse()?,
                Box::pin(async move { cancel_token.cancelled().await }),
            )
            .await?;
        Ok(())
    }
}
