use async_trait::async_trait;

use crate::domain::counter::Counter;

#[async_trait]
pub trait CounterRepository: Send + Sync {
    async fn increment(&self, key: &str) -> anyhow::Result<Counter>;
    async fn get(&self, key: &str) -> anyhow::Result<Counter>;
}
