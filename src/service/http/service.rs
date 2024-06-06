#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCli;
#[cfg(feature = "cli")]
use crate::api::cli::roadster::RoadsterCommand;
#[cfg(all(feature = "cli", feature = "open-api"))]
use crate::api::cli::roadster::RoadsterSubCommand;
use crate::app::App;
use crate::app_context::AppContext;
use crate::error::RoadsterResult;
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
impl<A: App + 'static> AppService<A> for HttpService {
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
    ) -> RoadsterResult<bool> {
        if let Some(command) = roadster_cli.command.as_ref() {
            match command {
                RoadsterCommand::Roadster(args) => match &args.command {
                    #[cfg(feature = "open-api")]
                    RoadsterSubCommand::ListRoutes(_) => {
                        info!("API routes:");
                        self.list_routes()
                            .iter()
                            .for_each(|(path, method)| info!("[{method}]\t{path}"));
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
        self: Box<Self>,
        app_context: &AppContext<A::State>,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        let server_addr = app_context.config().service.http.custom.address.url();
        info!("Http server will start at {server_addr}");

        let app_listener = tokio::net::TcpListener::bind(server_addr).await?;
        axum::serve(app_listener, self.router)
            .with_graceful_shutdown(Box::pin(async move { cancel_token.cancelled().await }))
            .await?;

        Ok(())
    }
}

impl HttpService {
    /// Create a new [HttpServiceBuilder].
    pub fn builder<A: App>(
        path_root: Option<&str>,
        context: &AppContext<A::State>,
    ) -> HttpServiceBuilder<A> {
        HttpServiceBuilder::new(path_root, context)
    }

    /// List the available HTTP API routes.
    #[cfg(feature = "open-api")]
    pub fn list_routes(&self) -> Vec<(&str, &str)> {
        self.api
            .as_ref()
            .operations()
            .sorted_by(|(path_a, _, _), (path_b, _, _)| Ord::cmp(&path_a, &path_b))
            .map(|(path, method, _)| (path, method))
            .collect()
    }

    /// Generate an OpenAPI schema for the HTTP API.
    #[cfg(feature = "open-api")]
    pub fn open_api_schema(
        &self,
        pretty_print: bool,
        output: Option<&PathBuf>,
    ) -> RoadsterResult<()> {
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

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "open-api")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn list_routes() {
        use super::*;
        use aide::axum::routing::{delete_with, get, get_with, post_with, put_with};
        use aide::axum::ApiRouter;
        use aide::openapi::OpenApi;
        use itertools::Itertools;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        async fn api_method() {}
        let mut open_api = OpenApi::default();
        let router = ApiRouter::new()
            .api_route("/foo", get_with(api_method, |op| op))
            .api_route("/bar", post_with(api_method, |op| op))
            .api_route("/baz", put_with(api_method, |op| op))
            .api_route("/a", delete_with(api_method, |op| op))
            .api_route("/c", get_with(api_method, |op| op))
            .api_route("/b", get_with(api_method, |op| op))
            // Methods registered with `get` instead of `get_with` will not have OpenAPI
            // documentation generated, but will still be included in the list of routes.
            .api_route("/not_documented", get(api_method))
            .finish_api(&mut open_api);

        let service = HttpService {
            router,
            api: Arc::new(open_api),
        };

        let paths = service
            .list_routes()
            .iter()
            .map(|(path, _)| path.to_string())
            .collect_vec();
        assert_eq!(
            paths,
            ["/a", "/b", "/bar", "/baz", "/c", "/foo", "/not_documented"]
                .iter()
                .map(|s| s.to_string())
                .collect_vec()
        );
        let paths: BTreeMap<&str, &str> = service.list_routes().into_iter().collect();
        assert_eq!(paths.get("/foo").unwrap(), &"get");
        assert_eq!(paths.get("/bar").unwrap(), &"post");
        assert_eq!(paths.get("/baz").unwrap(), &"put");
        assert_eq!(paths.get("/a").unwrap(), &"delete");
    }
}
