use crate::app::App;
use crate::app_context::AppContext;
#[cfg(all(feature = "cli", feature = "open-api"))]
use crate::cli::RoadsterSubCommand;
#[cfg(feature = "cli")]
use crate::cli::{RoadsterCli, RoadsterCommand};
use crate::service::http::builder::HttpServiceBuilder;
use crate::service::AppService;
#[cfg(feature = "open-api")]
use aide::openapi::OpenApi;
use async_trait::async_trait;
use axum::Router;
#[cfg(feature = "open-api")]
use itertools::Itertools;
#[cfg(feature = "open-api")]
use std::fs::File;
#[cfg(feature = "open-api")]
use std::io::Write;
#[cfg(feature = "open-api")]
use std::path::PathBuf;
#[cfg(feature = "open-api")]
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub struct HttpService {
    pub(crate) router: Router,
    #[cfg(feature = "open-api")]
    pub(crate) api: Arc<OpenApi>,
}

#[async_trait]
impl<A: App> AppService<A> for HttpService {
    fn name() -> String {
        "http".to_string()
    }

    fn enabled(context: &AppContext<A::State>) -> bool {
        context.config().service.http.common.enabled(context)
    }

    #[cfg(feature = "cli")]
    async fn handle_cli(
        &self,
        roadster_cli: &RoadsterCli,
        _app_cli: &A::Cli,
        _app_context: &AppContext<A::State>,
    ) -> anyhow::Result<bool> {
        if let Some(command) = roadster_cli.command.as_ref() {
            match command {
                RoadsterCommand::Roadster(args) => match &args.command {
                    #[cfg(feature = "open-api")]
                    RoadsterSubCommand::ListRoutes(_) => {
                        self.list_routes();
                        return Ok(true);
                    }
                    #[cfg(feature = "open-api")]
                    RoadsterSubCommand::OpenApi(args) => {
                        self.open_api_schema(args.pretty_print, args.output.as_ref())?;
                        return Ok(true);
                    }
                    _ => {}
                },
            }
        }
        Ok(false)
    }

    async fn run(
        &self,
        app_context: AppContext<A::State>,
        cancel_token: CancellationToken,
    ) -> anyhow::Result<()> {
        let server_addr = app_context.config().service.http.custom.address.url();
        info!("Server will start at {server_addr}");

        let app_listener = tokio::net::TcpListener::bind(server_addr).await?;
        axum::serve(app_listener, self.router.clone())
            .with_graceful_shutdown(Box::pin(async move { cancel_token.cancelled().await }))
            .await?;

        Ok(())
    }
}

impl HttpService {
    /// Create a new [HttpServiceBuilder].
    pub fn builder<A: App>(
        path_root: &str,
        context: &AppContext<A::State>,
    ) -> HttpServiceBuilder<A> {
        HttpServiceBuilder::new(path_root, context)
    }

    /// List the available HTTP API routes.
    #[cfg(feature = "open-api")]
    pub fn list_routes(&self) {
        info!("API routes:");
        self.api
            .as_ref()
            .operations()
            .sorted_by(|(path_a, _, _), (path_b, _, _)| Ord::cmp(&path_a, &path_b))
            .for_each(|(path, method, _operation)| info!("[{method}]\t{path}"));
    }

    /// Generate an OpenAPI schema for the HTTP API.
    #[cfg(feature = "open-api")]
    pub fn open_api_schema(
        &self,
        pretty_print: bool,
        output: Option<&PathBuf>,
    ) -> anyhow::Result<()> {
        let schema_json = if pretty_print {
            serde_json::to_string_pretty(self.api.as_ref())?
        } else {
            serde_json::to_string(self.api.as_ref())?
        };
        if let Some(path) = output {
            info!("Writing schema to {:?}", path);
            write!(File::create(path)?, "{schema_json}")?;
        } else {
            info!("OpenAPI schema:");
            info!("{schema_json}");
        };
        Ok(())
    }
}
