use crate::app::context::AppContext;
#[cfg(feature = "db-diesel")]
use crate::db::migration::diesel::DieselMigrator;
#[cfg(feature = "db-sea-orm")]
use crate::db::migration::sea_orm::SeaOrmMigrator;
use crate::db::migration::{DownArgs, MigrationInfo, Migrator, UpArgs};
use crate::error::RoadsterResult;
use async_trait::async_trait;
use axum_core::extract::FromRef;
#[cfg(feature = "db-diesel")]
use diesel::Connection;
#[cfg(feature = "db-diesel")]
use diesel_migrations::MigrationHarness;
use itertools::Itertools;
use std::any::type_name;
use std::collections::BTreeMap;
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MigratorRegistryError {
    /// The provided [`Migrator`] was already registered. Contains the
    /// type name of the provided service.
    #[error("The provided `Migrator` was already registered: `{0}`")]
    AlreadyRegistered(&'static str),

    #[error(transparent)]
    Other(#[from] Box<dyn Send + Sync + std::error::Error>),
}

/// Registry of [`Migrator`]s that will be run to set up the database.
pub struct MigratorRegistry<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    migrators: BTreeMap<&'static str, Box<dyn Migrator<S, Error = crate::error::Error>>>,
}

impl<S> MigratorRegistry<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) fn new() -> Self {
        Self {
            migrators: Default::default(),
        }
    }

    /// Register a new [`Migrator`].
    ///
    /// Note: SeaORM and Diesel migrations expect all of the applied migrations to be available
    /// to the provided migrator, so multiple SeaORM or Diesel migrators should not be provided
    /// to this [`MigratorRegistry`].
    pub fn register<M>(&mut self, migrator: M) -> RoadsterResult<()>
    where
        M: 'static + Migrator<S>,
    {
        self.register_wrapped(MigratorWrapper::new(migrator))
    }

    /// Register a new [`sea_orm_migration::MigratorTrait`].
    ///
    /// Note: SeaORM migrations expect all of the applied migrations to be available
    /// to the provided migrator, so this method should only be called once.
    #[cfg(feature = "db-sea-orm")]
    pub fn register_sea_orm_migrator<M>(&mut self, migrator: M) -> RoadsterResult<()>
    where
        M: 'static + Send + Sync + sea_orm_migration::MigratorTrait,
    {
        self.register_wrapped(MigratorWrapper::new(SeaOrmMigrator::new(migrator)))
    }

    /// Register a new [`diesel::migration::MigrationSource`].
    ///
    /// Note: Diesel migrations expect all of the applied migrations to be available
    /// to the provided migrator, so this method should only be called once.
    #[cfg(feature = "db-diesel")]
    pub fn register_diesel_migrator<C>(
        &mut self,
        migrator: impl 'static + Send + Sync + diesel::migration::MigrationSource<C::Backend>,
    ) -> RoadsterResult<()>
    where
        C: 'static + Send + Connection + MigrationHarness<C::Backend>,
    {
        self.register_wrapped(MigratorWrapper::new(DieselMigrator::<C>::new(migrator)))
    }

    pub(crate) fn register_wrapped(&mut self, migrator: MigratorWrapper<S>) -> RoadsterResult<()> {
        self.register_boxed(migrator.type_name, Box::new(migrator))
    }

    pub(crate) fn register_boxed(
        &mut self,
        type_name: &'static str,
        migrator: Box<dyn Migrator<S, Error = crate::error::Error>>,
    ) -> RoadsterResult<()> {
        if self.migrators.insert(type_name, migrator).is_some() {
            return Err(MigratorRegistryError::AlreadyRegistered(type_name).into());
        }

        Ok(())
    }

    pub fn migrators(&self) -> Vec<&dyn Migrator<S, Error = crate::error::Error>> {
        self.migrators
            .values()
            .map(|migrator| migrator.as_ref())
            .collect_vec()
    }
}

type UpFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
            &'a UpArgs,
        )
            -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<usize>>>>,
>;

type DownFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
            &'a DownArgs,
        )
            -> std::pin::Pin<Box<dyn 'a + Send + Future<Output = RoadsterResult<usize>>>>,
>;

type StatusFn<S> = Box<
    dyn Send
        + Sync
        + for<'a> Fn(
            &'a S,
        ) -> std::pin::Pin<
            Box<dyn 'a + Send + Future<Output = RoadsterResult<Vec<MigrationInfo>>>>,
        >,
>;

pub(crate) struct MigratorWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) type_name: &'static str,
    up_fn: UpFn<S>,
    down_fn: DownFn<S>,
    status_fn: StatusFn<S>,
}

impl<S> MigratorWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    pub(crate) fn new<M>(migrator: M) -> Self
    where
        M: 'static + Migrator<S>,
    {
        let type_name = type_name::<M>();
        let migrator = Arc::new(migrator);
        let up_fn: UpFn<S> = {
            let migrator = migrator.clone();
            Box::new(move |state, args| {
                let migrator = migrator.clone();
                Box::pin(async move {
                    let result = migrator
                        .up(state, args)
                        .await
                        .map_err(|err| MigratorRegistryError::Other(Box::new(err)))?;
                    Ok(result)
                })
            })
        };
        let down_fn: DownFn<S> = {
            let migrator = migrator.clone();
            Box::new(move |state, args| {
                let migrator = migrator.clone();
                Box::pin(async move {
                    let result = migrator
                        .down(state, args)
                        .await
                        .map_err(|err| MigratorRegistryError::Other(Box::new(err)))?;
                    Ok(result)
                })
            })
        };
        let status_fn: StatusFn<S> = {
            let migrator = migrator.clone();
            Box::new(move |state| {
                let migrator = migrator.clone();
                Box::pin(async move {
                    let result = migrator
                        .status(state)
                        .await
                        .map_err(|err| MigratorRegistryError::Other(Box::new(err)))?;
                    Ok(result)
                })
            })
        };
        Self {
            type_name,
            up_fn,
            down_fn,
            status_fn,
        }
    }
}

#[async_trait]
impl<S> Migrator<S> for MigratorWrapper<S>
where
    S: 'static + Send + Sync + Clone,
    AppContext: FromRef<S>,
{
    type Error = crate::error::Error;

    async fn up(&self, state: &S, args: &UpArgs) -> Result<usize, Self::Error> {
        (self.up_fn)(state, args).await
    }

    async fn down(&self, state: &S, args: &DownArgs) -> Result<usize, Self::Error> {
        (self.down_fn)(state, args).await
    }

    async fn status(&self, state: &S) -> Result<Vec<MigrationInfo>, Self::Error> {
        (self.status_fn)(state).await
    }
}
