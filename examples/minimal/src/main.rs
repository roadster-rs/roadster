use minimal::app::App;
use roadster::app;
use roadster::error::RoadsterResult;

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    app::run(App).await?;

    Ok(())
}
