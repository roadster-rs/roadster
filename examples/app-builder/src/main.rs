use app_builder::build_app;
use roadster::app;
use roadster::error::RoadsterResult;

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let app = build_app();

    app::run(app).await?;

    Ok(())
}
