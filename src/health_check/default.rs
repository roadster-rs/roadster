use crate::app::context::AppContext;
#[cfg(feature = "db-sql")]
use crate::health_check::database::DatabaseHealthCheck;
#[cfg(feature = "sidekiq")]
use crate::health_check::sidekiq_enqueue::SidekiqEnqueueHealthCheck;
#[cfg(feature = "sidekiq")]
use crate::health_check::sidekiq_fetch::SidekiqFetchHealthCheck;
use crate::health_check::HealthCheck;
use std::sync::Arc;

pub fn default_health_checks(
    #[allow(unused_variables)] context: &AppContext,
) -> Vec<Arc<dyn HealthCheck>> {
    vec![
        #[cfg(feature = "db-sql")]
        Arc::new(DatabaseHealthCheck {
            context: context.clone(),
        }),
        #[cfg(feature = "sidekiq")]
        Arc::new(SidekiqEnqueueHealthCheck {
            context: context.clone(),
        }),
        #[cfg(feature = "sidekiq")]
        Arc::new(SidekiqFetchHealthCheck {
            context: context.clone(),
        }),
    ]
}

#[cfg(all(test, feature = "sidekiq", feature = "db-sql",))]
mod tests {
    use crate::app::context::AppContext;
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

        let context = AppContext::test(Some(config), None, None).unwrap();

        // Act
        let health_checks = super::default_health_checks(&context);
        let health_checks = health_checks.iter().map(|check| check.name()).collect_vec();

        // Assert
        assert_toml_snapshot!(health_checks);
    }
}
