use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::Service;
use crate::service::http::builder::HttpServiceBuilder;
#[cfg(feature = "open-api")]
use aide::openapi::OpenApi;
use async_trait::async_trait;
use axum::Router;
use axum_core::extract::FromRef;
#[cfg(feature = "open-api")]
use itertools::Itertools;
#[cfg(feature = "open-api")]
use std::fs::File;
#[cfg(feature = "open-api")]
use std::io::Write;
use std::path::PathBuf;
#[cfg(feature = "open-api")]
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::info;

pub(crate) const NAME: &str = "http";

pub(crate) fn enabled(context: &AppContext) -> bool {
    context.config().service.http.common.enabled(context)
}

pub struct HttpService {
    pub(crate) router: Router,
    #[cfg(feature = "open-api")]
    pub(crate) api: Arc<OpenApi>,
}

#[async_trait]
impl<S> Service<S> for HttpService
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        NAME.to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        enabled(&AppContext::from_ref(state))
    }

    async fn run(
        self: Box<Self>,
        state: &S,
        cancel_token: CancellationToken,
    ) -> RoadsterResult<()> {
        let server_addr = AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .address
            .url();
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
    pub fn builder<S>(path_root: Option<&str>, state: &S) -> HttpServiceBuilder<S>
    where
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
    {
        HttpServiceBuilder::new(path_root, state)
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    /// List the available HTTP API routes.
    ///
    /// Note that in order for a route to show up where, it needs to be registered with an
    /// [`aide::axum::ApiRouter`] and provided when building the [`HttpService`].
    #[cfg(feature = "open-api")]
    pub fn list_routes(&self) -> Vec<(&str, &str)> {
        self.api
            .as_ref()
            .operations()
            .sorted_by(|(path_a, _, _), (path_b, _, _)| Ord::cmp(&path_a, &path_b))
            .map(|(path, method, _)| (path, method))
            .collect()
    }

    /// Generate an OpenAPI schema for the HTTP API and either print to stdout or to the path
    /// provided in [`OpenApiArgs`].
    #[cfg(feature = "open-api")]
    pub fn print_open_api_schema(&self, options: &OpenApiArgs) -> RoadsterResult<()> {
        let schema = self.open_api_schema(options)?;
        if let Some(path) = &options.output {
            info!("Writing schema to {:?}", path);
            write!(File::create(path)?, "{schema}")?;
        } else {
            info!("OpenAPI schema:");
            info!("{schema}");
        };
        Ok(())
    }

    /// Generate an OpenAPI schema for the HTTP API and either return it as a JSON string.
    ///
    /// Note: This method ignores the `output` field of [`OpenApiArgs`].
    #[cfg(feature = "open-api")]
    pub fn open_api_schema(&self, options: &OpenApiArgs) -> RoadsterResult<String> {
        let schema = if options.pretty_print {
            serde_json::to_string_pretty(self.api.as_ref())?
        } else {
            serde_json::to_string(self.api.as_ref())?
        };
        Ok(schema)
    }

    /// Get the [`OpenApi`] for this [`HttpService`]. Useful to implement custom processing
    /// of the schema that isn't provided by Roadster.
    #[cfg(feature = "open-api")]
    pub fn open_api(&self) -> Arc<OpenApi> {
        self.api.clone()
    }
}

#[derive(Debug, serde_derive::Serialize, typed_builder::TypedBuilder)]
#[cfg_attr(feature = "cli", derive(clap::Parser))]
#[non_exhaustive]
pub struct OpenApiArgs {
    /// The file to write the schema to. If not provided, will write to stdout.
    #[builder(default, setter(strip_option))]
    #[cfg_attr(feature = "cli", clap(short, long, value_name = "FILE", value_hint = clap::ValueHint::FilePath))]
    pub output: Option<PathBuf>,

    /// Whether to pretty-print the schema. Default: false.
    #[cfg_attr(feature = "cli", clap(short, long, default_value_t = false))]
    #[builder(default)]
    pub pretty_print: bool,
}

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(feature = "open-api")]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn list_routes() {
        use super::*;
        use aide::axum::ApiRouter;
        use aide::axum::routing::{delete_with, get, get_with, post_with, put_with};
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
