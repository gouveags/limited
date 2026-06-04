use async_trait::async_trait;

use crate::{
    domain::counter::Counter, infrastructure::redis::RedisInfrastructure,
    ports::counter_repository::CounterRepository,
};

#[derive(Clone)]
pub struct RedisCounterRepository {
    redis: RedisInfrastructure,
}

impl RedisCounterRepository {
    pub fn new(redis: RedisInfrastructure) -> Self {
        Self { redis }
    }

    fn key(key: &str) -> String {
        format!("counter:{key}")
    }
}

#[async_trait]
impl CounterRepository for RedisCounterRepository {
    async fn increment(&self, key: &str) -> anyhow::Result<Counter> {
        let mut connection = self.redis.connection().await?;
        let value: u64 = redis::cmd("INCR")
            .arg(Self::key(key))
            .query_async(&mut connection)
            .await?;

        Ok(Counter {
            key: key.to_string(),
            value,
        })
    }

    async fn get(&self, key: &str) -> anyhow::Result<Counter> {
        let mut connection = self.redis.connection().await?;
        let value: Option<u64> = redis::cmd("GET")
            .arg(Self::key(key))
            .query_async(&mut connection)
            .await?;

        Ok(Counter {
            key: key.to_string(),
            value: value.unwrap_or(0),
        })
    }
}
