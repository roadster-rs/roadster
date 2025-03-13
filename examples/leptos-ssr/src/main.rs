#[cfg(feature = "ssr")]
use roadster::error::RoadsterResult;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() -> RoadsterResult<()> {
    use leptos_ssr_example::server::build_app;
    use roadster::app;

    app::run(build_app()).await?;

    Ok(())
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
