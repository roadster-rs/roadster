use crate::app::context::AppContext;
use crate::worker::backend::multi::MultiBackend;
use crate::worker::backend::pg::PgBackend;
use crate::worker::enqueue::enqueue_config;
use crate::worker::{Enqueuer, QueueBackend, Worker};
use async_trait::async_trait;
use axum_core::extract::FromRef;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;

#[async_trait]
impl Enqueuer for MultiBackend {
    type Error = crate::error::Error;

    #[instrument(skip_all)]
    async fn enqueue<W, S, Args, E>(state: &S, args: &Args) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        let enqueue_config = enqueue_config::<W, _, _, _>(state)?;
        match enqueue_config.backend {
            QueueBackend::Sidekiq => {
                todo!()
            }
            QueueBackend::Pg => PgBackend::enqueue::<W, _, _, _>(state, args),
            QueueBackend::Other(_) => {
                todo!()
            }
        }
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_delayed<W, S, Args, E>(
        state: &S,
        args: &Args,
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        let enqueue_config = enqueue_config::<W, _, _, _>(state)?;
        match enqueue_config.backend {
            QueueBackend::Sidekiq => {
                todo!()
            }
            QueueBackend::Pg => PgBackend::enqueue_delayed::<W, _, _, _>(state, args, delay),
            QueueBackend::Other(_) => {
                todo!()
            }
        }
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch<W, S, Args, E>(state: &S, args: &[Args]) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        let enqueue_config = enqueue_config::<W, _, _, _>(state)?;
        match enqueue_config.backend {
            QueueBackend::Sidekiq => {
                todo!()
            }
            QueueBackend::Pg => PgBackend::enqueue_batch::<W, _, _, _>(state, args),
            QueueBackend::Other(_) => {
                todo!()
            }
        }
        .await
    }

    #[instrument(skip_all)]
    async fn enqueue_batch_delayed<W, S, Args, E>(
        state: &S,
        args: &[Args],
        delay: Duration,
    ) -> Result<(), Self::Error>
    where
        W: 'static + Worker<S, Args, Error = E>,
        S: Clone + Send + Sync + 'static,
        AppContext: FromRef<S>,
        Args: Send + Sync + Serialize + for<'de> Deserialize<'de>,
    {
        let enqueue_config = enqueue_config::<W, _, _, _>(state)?;
        match enqueue_config.backend {
            QueueBackend::Sidekiq => {
                todo!()
            }
            QueueBackend::Pg => PgBackend::enqueue_batch_delayed::<W, _, _, _>(state, args, delay),
            QueueBackend::Other(_) => {
                todo!()
            }
        }
        .await
    }
}
