use pg_worker::build_app;
use roadster::app::run;
use roadster::error::RoadsterResult;

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    let app = build_app();

    run(app).await?;

    Ok(())
}
