use migration::Migrator;
use minimal::app::App;
use roadster::app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::start::<App, Migrator>().await?;

    Ok(())
}
