use crate::worker::sidekiq::worker::ExampleWorker;
use axum::extract::State;
use roadster::app::context::AppContext;
use roadster::error::RoadsterResult;
use roadster::worker::Worker;

async fn example_get(State(state): State<AppContext>) -> RoadsterResult<()> {
    // Enqueue the job in your API handler
    ExampleWorker::enqueue(&state, "Example".to_string()).await?;

    Ok(())
}
