use anyhow::Context;
use redis::aio::MultiplexedConnection;

#[derive(Clone)]
pub struct RedisInfrastructure {
    client: redis::Client,
}

impl RedisInfrastructure {
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let client = redis::Client::open(url).context("invalid Redis URL")?;
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect to Redis")?;

        let _: String = redis::cmd("PING")
            .query_async(&mut connection)
            .await
            .context("failed to ping Redis")?;

        Ok(Self { client })
    }

    pub async fn connection(&self) -> anyhow::Result<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .context("failed to connect to Redis")
    }
}
