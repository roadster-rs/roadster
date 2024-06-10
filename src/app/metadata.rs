use typed_builder::TypedBuilder;

/// Metadata for the app. This is provided separately from the
/// [AppConfig][crate::config::app_config::AppConfig] in order to allow the consumer to provide
/// metadata, such as the app version, that is best determined dynamically.
#[derive(Debug, Default, Clone, TypedBuilder)]
#[non_exhaustive]
pub struct AppMetadata {
    /// The name of the app. If not provided, Roadster will use the value from
    /// the [config][crate::config::app_config::App].
    #[builder(default, setter(strip_option))]
    pub name: Option<String>,
    /// The version of the app. For example, the cargo package version or the git commit sha.
    #[builder(default, setter(strip_option))]
    pub version: Option<String>,
}
