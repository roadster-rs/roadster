use full::app::App;
use roadster::app::run;
use roadster::error::RoadsterResult;

#[tokio::main]
async fn main() -> RoadsterResult<()> {
    run(App).await?;

    Ok(())
}
