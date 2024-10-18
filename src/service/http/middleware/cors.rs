use crate::app::context::AppContext;
use crate::error::RoadsterResult;
use crate::service::http::middleware::Middleware;
use axum::http::{HeaderName, HeaderValue, Method};
use axum::Router;
use axum_core::extract::FromRef;
use itertools::Itertools;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none};
use std::str::FromStr;
use std::time::Duration;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer, ExposeHeaders};
use validator::Validate;

#[serde_as]
#[skip_serializing_none]
#[derive(Debug, Clone, Validate, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub struct CorsConfig {
    #[serde(default)]
    pub preset: CorsPreset,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.allow_credentials>
    #[serde(default)]
    pub allow_credentials: Option<bool>,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.allow_private_network>
    #[serde(default)]
    pub allow_private_network: Option<bool>,

    /// Duration in milliseconds. If a value less than one second (1000 ms) is provided, the
    /// header will not be set by the middleware.
    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.max_age>
    #[serde(default = "default_max_age")]
    #[serde_as(as = "serde_with::DurationMilliSeconds")]
    pub max_age: Duration,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.allow_headers>
    #[serde(default)]
    pub allow_headers: Option<CorsAllowHeaders>,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.allow_methods>
    #[serde(default)]
    pub allow_methods: Option<CorsAllowMethods>,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.allow_origin>
    #[serde(default)]
    pub allow_origins: Option<CorsAllowOrigins>,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.expose_headers>
    #[serde(default)]
    pub expose_headers: Option<CorsExposeHeaders>,

    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.vary>
    // Todo: deserialize as HeaderName directly instead of string
    #[serde(default)]
    pub vary: Option<Vec<String>>,
}

fn default_max_age() -> Duration {
    Duration::from_secs(60 * 60)
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CorsPreset {
    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.new>
    #[default]
    Restrictive,
    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.permissive>
    Permissive,
    /// See <https://docs.rs/tower-http/latest/tower_http/cors/struct.CorsLayer.html#method.very_permissive>
    VeryPermissive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CorsAllowHeaders {
    Any,
    MirrorRequest,
    // Todo: deserialize as HeaderName directly instead of string
    List { headers: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CorsAllowMethods {
    Any,
    MirrorRequest,
    // Todo: deserialize as Method directly instead of string
    Exact { method: String },
    // Todo: deserialize as Method directly instead of string
    List { methods: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CorsAllowOrigins {
    Any,
    MirrorRequest,
    // Todo: deserialize as HeaderValue directly instead of string
    Exact { origin: String },
    // Todo: deserialize as HeaderValue directly instead of string
    List { origins: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum CorsExposeHeaders {
    Any,
    // Todo: deserialize as HeaderName directly instead of string
    List { headers: Vec<String> },
}

fn parse_header_names(header_names: &[String]) -> RoadsterResult<Vec<HeaderName>> {
    let header_names = header_names
        .iter()
        .map(|header_name| HeaderName::from_str(header_name))
        .try_collect()?;
    Ok(header_names)
}

fn parse_header_values(header_values: &[String]) -> RoadsterResult<Vec<HeaderValue>> {
    let header_values = header_values
        .iter()
        .map(|header_value| HeaderValue::from_str(header_value))
        .try_collect()?;
    Ok(header_values)
}

fn parse_methods(methods: &[String]) -> RoadsterResult<Vec<Method>> {
    let methods = methods
        .iter()
        .map(|method| Method::from_str(method))
        .try_collect()?;
    Ok(methods)
}

pub struct CorsMiddleware;
impl<S> Middleware<S> for CorsMiddleware
where
    S: Clone + Send + Sync + 'static,
    AppContext: FromRef<S>,
{
    fn name(&self) -> String {
        "cors".to_string()
    }

    fn enabled(&self, state: &S) -> bool {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .cors
            .common
            .enabled(state)
    }

    fn priority(&self, state: &S) -> i32 {
        AppContext::from_ref(state)
            .config()
            .service
            .http
            .custom
            .middleware
            .cors
            .common
            .priority
    }

    fn install(&self, router: Router, state: &S) -> RoadsterResult<Router> {
        let context = AppContext::from_ref(state);
        let config = &context.config().service.http.custom.middleware.cors.custom;
        let layer = match config.preset {
            CorsPreset::Restrictive => CorsLayer::new(),
            CorsPreset::Permissive => CorsLayer::permissive(),
            CorsPreset::VeryPermissive => CorsLayer::very_permissive(),
        };

        let layer = if config.max_age > Duration::from_secs(1) {
            layer.max_age(config.max_age)
        } else {
            layer
        };

        let layer = config
            .allow_credentials
            .iter()
            .fold(layer, |layer, allow| layer.allow_credentials(*allow));

        let layer = config
            .allow_private_network
            .iter()
            .fold(layer, |layer, allow| layer.allow_private_network(*allow));

        let layer = config.allow_headers.iter().try_fold(
            layer,
            |layer, allow| -> RoadsterResult<CorsLayer> {
                let layer = match allow {
                    CorsAllowHeaders::Any => layer.allow_headers(AllowHeaders::any()),
                    CorsAllowHeaders::MirrorRequest => {
                        layer.allow_headers(AllowHeaders::mirror_request())
                    }
                    CorsAllowHeaders::List { headers } => {
                        layer.allow_headers(AllowHeaders::list(parse_header_names(headers)?))
                    }
                };
                Ok(layer)
            },
        )?;

        let layer = config.expose_headers.iter().try_fold(
            layer,
            |layer, allow| -> RoadsterResult<CorsLayer> {
                let layer = match allow {
                    CorsExposeHeaders::Any => layer.expose_headers(ExposeHeaders::any()),
                    CorsExposeHeaders::List { headers } => {
                        layer.expose_headers(ExposeHeaders::list(parse_header_names(headers)?))
                    }
                };
                Ok(layer)
            },
        )?;

        let layer = config.vary.iter().try_fold(
            layer,
            |layer, header_names| -> RoadsterResult<CorsLayer> {
                let layer = layer.vary(parse_header_names(header_names)?);
                Ok(layer)
            },
        )?;

        let layer = config.allow_origins.iter().try_fold(
            layer,
            |layer, allow| -> RoadsterResult<CorsLayer> {
                let layer = match allow {
                    CorsAllowOrigins::Any => layer.allow_origin(AllowOrigin::any()),
                    CorsAllowOrigins::MirrorRequest => {
                        layer.allow_origin(AllowOrigin::mirror_request())
                    }
                    CorsAllowOrigins::Exact { origin } => {
                        layer.allow_origin(AllowOrigin::exact(HeaderValue::from_str(origin)?))
                    }
                    CorsAllowOrigins::List { origins } => {
                        layer.allow_origin(AllowOrigin::list(parse_header_values(origins)?))
                    }
                };
                Ok(layer)
            },
        )?;

        let layer = config.allow_methods.iter().try_fold(
            layer,
            |layer, allow| -> RoadsterResult<CorsLayer> {
                let layer = match allow {
                    CorsAllowMethods::Any => layer.allow_methods(AllowMethods::any()),
                    CorsAllowMethods::MirrorRequest => {
                        layer.allow_methods(AllowMethods::mirror_request())
                    }
                    CorsAllowMethods::Exact { method } => {
                        layer.allow_methods(AllowMethods::exact(Method::from_str(method)?))
                    }
                    CorsAllowMethods::List { methods } => {
                        layer.allow_methods(AllowMethods::list(parse_methods(methods)?))
                    }
                };
                Ok(layer)
            },
        )?;

        let router = router.layer(layer);

        Ok(router)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::testing::snapshot::TestCase;
    use crate::util::serde::Wrapper;
    use insta::assert_toml_snapshot;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(false, Some(true), true)]
    #[case(false, Some(false), false)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn cors_enabled(
        #[case] default_enable: bool,
        #[case] enable: Option<bool>,
        #[case] expected_enabled: bool,
    ) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.service.http.custom.middleware.default_enable = default_enable;
        config.service.http.custom.middleware.cors.common.enable = enable;

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = CorsMiddleware;

        // Act/Assert
        assert_eq!(middleware.enabled(&context), expected_enabled);
    }

    #[rstest]
    #[case(None, -9950)]
    #[case(Some(1234), 1234)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn cors_priority(#[case] override_priority: Option<i32>, #[case] expected_priority: i32) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        if let Some(priority) = override_priority {
            config.service.http.custom.middleware.cors.common.priority = priority;
        }

        let context = AppContext::test(Some(config), None, None).unwrap();

        let middleware = CorsMiddleware;

        // Act/Assert
        assert_eq!(middleware.priority(&context), expected_priority);
    }

    #[rstest]
    #[case(
        r#"
        [inner]
        type = 'any'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'mirror-request'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'list'
        headers = ["foo", "bar"]
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_cors_allow_headers(_case: TestCase, #[case] serialized: &str) {
        let value: Wrapper<CorsAllowHeaders> = toml::from_str(serialized).unwrap();
        assert_toml_snapshot!(value);
    }

    #[rstest]
    #[case(
        r#"
        [inner]
        type = 'any'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'mirror-request'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'exact'
        method = "foo"
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'list'
        methods = ["foo", "bar"]
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_cors_allow_methods(_case: TestCase, #[case] serialized: &str) {
        let value: Wrapper<CorsAllowMethods> = toml::from_str(serialized).unwrap();
        assert_toml_snapshot!(value);
    }

    #[rstest]
    #[case(
        r#"
        [inner]
        type = 'any'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'mirror-request'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'exact'
        origin = "foo"
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'list'
        origins = ["foo", "bar"]
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_cors_allow_origins(_case: TestCase, #[case] serialized: &str) {
        let value: Wrapper<CorsAllowOrigins> = toml::from_str(serialized).unwrap();
        assert_toml_snapshot!(value);
    }

    #[rstest]
    #[case(
        r#"
        [inner]
        type = 'any'
        "#
    )]
    #[case(
        r#"
        [inner]
        type = 'list'
        headers = ["foo", "bar"]
        "#
    )]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn deserialize_cors_expose_headers(_case: TestCase, #[case] serialized: &str) {
        let value: Wrapper<CorsExposeHeaders> = toml::from_str(serialized).unwrap();
        assert_toml_snapshot!(value);
    }
}
