use async_trait::async_trait;
use bb8::PooledConnection;
use sidekiq::redis_rs::ToRedisArgs;
use sidekiq::{RedisConnection, RedisConnectionManager, RedisError};

/// Trait to help with mocking responses from Redis.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub(crate) trait RedisCommands {
    async fn zrange(
        &mut self,
        key: String,
        lower: isize,
        upper: isize,
    ) -> Result<Vec<String>, RedisError>;

    async fn zrem<V>(&mut self, key: String, value: V) -> Result<usize, RedisError>
    where
        V: 'static + Send + Sync + ToRedisArgs;
}

#[async_trait]
impl RedisCommands for PooledConnection<'_, RedisConnectionManager> {
    async fn zrange(
        &mut self,
        key: String,
        lower: isize,
        upper: isize,
    ) -> Result<Vec<String>, RedisError> {
        RedisConnection::zrange(self, key, lower, upper).await
    }

    async fn zrem<V>(&mut self, key: String, value: V) -> Result<usize, RedisError>
    where
        V: 'static + Send + Sync + ToRedisArgs,
    {
        RedisConnection::zrem(self, key, value).await
    }
}
