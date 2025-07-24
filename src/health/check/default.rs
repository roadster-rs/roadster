use crate::app::context::AppContext;
use crate::health::check::HealthCheck;
#[cfg(feature = "db-diesel-mysql-pool-async")]
use crate::health::check::db::diesel_mysql_async::DbDieselMysqlAsyncHealthCheck;
#[cfg(feature = "db-diesel-postgres-pool-async")]
use crate::health::check::db::diesel_pg_async::DbDieselPgAsyncHealthCheck;
#[cfg(feature = "db-sea-orm")]
use crate::health::check::db::sea_orm::DbSeaOrmHealthCheck;
#[cfg(feature = "email-smtp")]
use crate::health::check::email::smtp::SmtpHealthCheck;
#[cfg(feature = "worker-pg")]
use crate::health::check::worker::pg::PgWorkerHealthCheck;
#[cfg(feature = "worker-sidekiq")]
use crate::health::check::worker::sidekiq::sidekiq_enqueue::SidekiqEnqueueHealthCheck;
#[cfg(feature = "worker-sidekiq")]
use crate::health::check::worker::sidekiq::sidekiq_fetch::SidekiqFetchHealthCheck;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn default_health_checks(
    #[allow(unused_variables)] context: &AppContext,
) -> BTreeMap<String, Arc<dyn HealthCheck>> {
    let health_checks: Vec<Arc<dyn HealthCheck>> = vec![
        #[cfg(feature = "db-sea-orm")]
        Arc::new(DbSeaOrmHealthCheck {
            context: context.downgrade(),
        }),
        #[cfg(feature = "db-diesel-postgres-pool")]
        Arc::new(crate::health::check::db::diesel::DbDieselHealthCheck::new(
            context,
            "postgres",
            |context| context.diesel_pg_pool(),
        )),
        #[cfg(feature = "db-diesel-mysql-pool")]
        Arc::new(crate::health::check::db::diesel::DbDieselHealthCheck::new(
            context,
            "mysql",
            |context| context.diesel_mysql_pool(),
        )),
        #[cfg(feature = "db-diesel-sqlite-pool")]
        Arc::new(crate::health::check::db::diesel::DbDieselHealthCheck::new(
            context,
            "sqlite",
            |context| context.diesel_sqlite_pool(),
        )),
        #[cfg(feature = "db-diesel-postgres-pool-async")]
        Arc::new(DbDieselPgAsyncHealthCheck {
            context: context.downgrade(),
        }),
        #[cfg(feature = "db-diesel-mysql-pool-async")]
        Arc::new(DbDieselMysqlAsyncHealthCheck {
            context: context.downgrade(),
        }),
        #[cfg(feature = "worker-sidekiq")]
        Arc::new(SidekiqEnqueueHealthCheck {
            context: context.downgrade(),
        }),
        #[cfg(feature = "worker-sidekiq")]
        Arc::new(SidekiqFetchHealthCheck {
            context: context.downgrade(),
        }),
        #[cfg(feature = "worker-pg")]
        Arc::new(PgWorkerHealthCheck {
            context: context.downgrade(),
        }),
        #[cfg(feature = "email-smtp")]
        Arc::new(SmtpHealthCheck {
            context: context.downgrade(),
        }),
    ];

    health_checks
        .into_iter()
        .filter(|check| check.enabled())
        .map(|check| (check.name(), check))
        .collect()
}

#[cfg(all(
    test,
    feature = "worker-sidekiq",
    feature = "worker-pg",
    feature = "db-sea-orm",
    feature = "db-diesel-postgres-pool",
    feature = "db-diesel-mysql-pool",
    feature = "db-diesel-sqlite-pool",
    feature = "db-diesel-postgres-pool-async",
    feature = "db-diesel-mysql-pool-async",
    feature = "email-smtp"
))]
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
    async fn default_health_checks(_case: TestCase, #[case] default_enable: bool) {
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
