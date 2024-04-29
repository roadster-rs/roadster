use crate::app::App;
use crate::app_context::AppContext;
#[cfg(feature = "cli")]
use crate::cli::RoadsterCli;
use async_trait::async_trait;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub mod http;

// Todo: add doc comments
// Todo: add/re-arrange app config fields to allow configuring services via app config
#[async_trait]
pub trait AppService<A: App>: Send + Sync {
    #[cfg(feature = "cli")]
    async fn handle_cli(
        &self,
        _roadster_cli: &RoadsterCli,
        _app_cli: &A::Cli,
        _app_context: &AppContext,
        _app_state: &A::State,
    ) -> anyhow::Result<bool> {
        Ok(false)
    }

    async fn run(
        &self,
        app_context: Arc<AppContext>,
        app_state: Arc<A::State>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()>;
}
