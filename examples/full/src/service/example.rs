use crate::app_state::AppState;
use roadster::error::RoadsterResult;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub async fn example_service(
    _state: AppState,
    _cancel_token: CancellationToken,
) -> RoadsterResult<()> {
    info!("Running example function-based service");
    Ok(())
}
