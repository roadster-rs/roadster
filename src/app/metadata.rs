/// Metadata for the app. This is provided separately from the
/// [`AppConfig`][crate::config::AppConfig] in order to allow the consumer to provide
/// metadata, such as the app version, that is best determined dynamically.
#[derive(Debug, Default, Clone, bon::Builder)]
#[non_exhaustive]
pub struct AppMetadata {
    /// The name of the app. If not provided, Roadster will use the value from
    /// the [config][crate::config::App].
    #[builder(into)]
    pub name: Option<String>,
    /// The version of the app. For example, the cargo package version or the git commit sha.
    #[builder(into)]
    pub version: Option<String>,
}
