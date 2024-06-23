use crate::app::context::AppContext;
use crate::app::App;
#[cfg(feature = "db-sql")]
use crate::health_check::database::DatabaseHealthCheck;
#[cfg(feature = "sidekiq")]
use crate::health_check::sidekiq::SidekiqHealthCheck;
use crate::health_check::HealthCheck;
use std::collections::BTreeMap;

pub fn default_health_checks<A: App + 'static>(
    context: &AppContext<A::State>,
) -> BTreeMap<String, Box<dyn HealthCheck<A>>> {
    let health_check: Vec<Box<dyn HealthCheck<A>>> = vec![
        #[cfg(feature = "db-sql")]
        Box::new(DatabaseHealthCheck),
        #[cfg(feature = "sidekiq")]
        Box::new(SidekiqHealthCheck),
    ];
    health_check
        .into_iter()
        .filter(|health_check| health_check.enabled(context))
        .map(|health_check| (health_check.name(), health_check))
        .collect()
}

#[cfg(all(test, feature = "sidekiq", feature = "db-sql",))]
mod tests {
    use crate::app::context::AppContext;
    use crate::app::MockApp;
    use crate::config::app_config::AppConfig;
    use crate::util::test_util::TestCase;
    use insta::assert_toml_snapshot;
    use itertools::Itertools;
    use rstest::{fixture, rstest};

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn default_middleware(_case: TestCase, #[case] default_enable: bool) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.health_check.default_enable = default_enable;

        let context = AppContext::<()>::test(Some(config), None, None).unwrap();

        // Act
        let health_checks = super::default_health_checks::<MockApp>(&context);
        let health_checks = health_checks.keys().collect_vec();

        // Assert
        assert_toml_snapshot!(health_checks);
    }
}
