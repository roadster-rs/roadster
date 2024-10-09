use crate::app::context::AppContext;
#[cfg(feature = "db-sql")]
use crate::health_check::database::DatabaseHealthCheck;
#[cfg(feature = "email-smtp")]
use crate::health_check::email::smtp::SmtpHealthCheck;
#[cfg(feature = "sidekiq")]
use crate::health_check::sidekiq_enqueue::SidekiqEnqueueHealthCheck;
#[cfg(feature = "sidekiq")]
use crate::health_check::sidekiq_fetch::SidekiqFetchHealthCheck;
use crate::health_check::HealthCheck;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn default_health_checks(
    #[allow(unused_variables)] context: &AppContext,
) -> BTreeMap<String, Arc<dyn HealthCheck>> {
    let health_checks: Vec<Arc<dyn HealthCheck>> = vec![
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
        #[cfg(feature = "email-smtp")]
        Arc::new(SmtpHealthCheck {
            context: context.clone(),
        }),
    ];

    health_checks
        .into_iter()
        .filter(|check| check.enabled())
        .map(|check| (check.name(), check))
        .collect()
}

#[cfg(all(test, feature = "sidekiq", feature = "db-sql", feature = "email-smtp"))]
mod tests {
    use crate::app::context::AppContext;
    use crate::config::AppConfig;
    use crate::testing::snapshot::TestCase;
    use bb8::Pool;
    use insta::assert_toml_snapshot;
    use itertools::Itertools;
    use rstest::{fixture, rstest};
    use sidekiq::RedisConnectionManager;

    #[fixture]
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn case() -> TestCase {
        Default::default()
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    #[tokio::test]
    #[cfg_attr(coverage_nightly, coverage(off))]
    async fn default_middleware(_case: TestCase, #[case] default_enable: bool) {
        // Arrange
        let mut config = AppConfig::test(None).unwrap();
        config.health_check.default_enable = default_enable;

        let redis_fetch = RedisConnectionManager::new("redis://invalid_host:1234").unwrap();
        let pool = Pool::builder().build_unchecked(redis_fetch);

        let context = AppContext::test(Some(config), None, Some(pool)).unwrap();

        // Act
        let health_checks = super::default_health_checks(&context);
        let health_checks = health_checks.keys().collect_vec();

        // Assert
        assert_toml_snapshot!(health_checks);
    }
}
