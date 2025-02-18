/// A placeholder that implements various traits so it can be used as the default for various type
/// parameters
pub struct Empty;

// Note: Unfortunately, this can't be implemented for any `impl App` because of a loop in the
// type resolution logic. So, just implement for the concrete `RoadsterApp` for now. If Rust's
// logic is ever updated to allow this type resolution loop, then we can add an `A: App` type
// parameter to implement for any `impl App`.
#[cfg(feature = "cli")]
#[async_trait::async_trait]
impl<S> crate::api::cli::RunCommand<crate::app::RoadsterApp<S, Empty>, S> for Empty
where
    S: Clone + Send + Sync + 'static,
    crate::app::context::AppContext: axum_core::extract::FromRef<S>,
{
    async fn run(
        &self,
        _prepared_app: &crate::app::PreparedApp<crate::app::RoadsterApp<S, Empty>, S>,
    ) -> crate::error::RoadsterResult<bool> {
        Ok(false)
    }
}

#[cfg(feature = "cli")]
impl clap::Args for Empty {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        cmd
    }
}

#[cfg(feature = "cli")]
impl clap::FromArgMatches for Empty {
    fn from_arg_matches(_matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        Ok(Empty)
    }

    fn update_from_arg_matches(&mut self, _matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        Ok(())
    }
}

#[cfg(feature = "db-sea-orm")]
impl sea_orm_migration::MigratorTrait for Empty {
    fn migrations() -> Vec<Box<dyn sea_orm_migration::MigrationTrait>> {
        Default::default()
    }
}
