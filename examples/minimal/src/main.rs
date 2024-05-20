use minimal::app::App;
use roadster::app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run(App).await?;

    Ok(())
}
