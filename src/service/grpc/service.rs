use crate::app::App;
use crate::app_context::AppContext;
use crate::error::RoadsterResult;
use crate::service::AppService;
use anyhow::anyhow;
use async_trait::async_trait;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tonic::transport::server::Router;
use tracing::info;

/// Simple wrapper around a tonic [Router] to run a gRPC service.
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
impl<A: App + 'static> AppService<A> for GrpcService {
    fn name() -> String
    where
        Self: Sized,
    {
        "grpc".to_string()
    }

    fn enabled(context: &AppContext<A::State>) -> bool
    where
        Self: Sized,
    {
        context.config().service.grpc.common.enabled(context)
    }

    async fn run(
        self: Box<Self>,
        app_context: &AppContext<A::State>,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        let server_addr = app_context.config().service.grpc.custom.address.url();
        info!("gRPC server will start at {server_addr}");

        self.router
            .into_inner()
            .unwrap()
            .serve_with_shutdown(
                server_addr
                    .parse()
                    .map_err(|err| anyhow!("Unable to parse server address: {}", err))?,
                Box::pin(async move { cancel_token.cancelled().await }),
            )
            .await?;
        Ok(())
    }
}
