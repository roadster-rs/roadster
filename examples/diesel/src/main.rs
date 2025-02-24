use roadster::app::run;
use roadster::error::RoadsterResult;
use roadster_diesel_example::build_app;

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let app = build_app();

    run(app).await?;

    Ok(())
}
