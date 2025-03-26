use cli_example::build_app;
use roadster::app::run;
use roadster::error::RoadsterResult;

#[tokio::main]
pub async fn main() -> RoadsterResult<()> {
    let app = build_app();
    run(app).await?;
    Ok(())
}
