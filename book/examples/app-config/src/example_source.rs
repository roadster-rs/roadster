use roadster::app::context::AppContext;
use roadster::app::{PrepareOptions, PreparedApp, RoadsterApp, prepare};
use roadster::config::ConfigOverrideSource;
use roadster::error::RoadsterResult;

type App = RoadsterApp<AppContext>;

async fn prepare_app(app: App) -> RoadsterResult<PreparedApp<App, AppContext>> {
    /*
    Config fields can be set using the name of the field, where each level in the config
    is separated by a `.`

    For example, `service.sidekiq.redis.uri` overrides the `AppConfig#service#sidekiq#redis#uri` field.
    See: <https://docs.rs/roadster/latest/roadster/config/service/worker/sidekiq/struct.Redis.html#structfield.uri>

    Note: Take care to not hard-code any sensitive values when providing a custom config source.
    However, it may be okay to hard-code a generic local connection URI (as we're doing here) if
    it's only used for testing (the primary intended purpose of allowing custom `Source`s).
     */
    let options = PrepareOptions::builder()
        .add_config_source(
            ConfigOverrideSource::builder()
                .name("service.sidekiq.redis.uri")
                .value("redis://localhost:6379")
                .build(),
        )
        .build();
    let app = prepare(app, options).await?;
    Ok(app)
}
