use async_trait::async_trait;

use crate::domain::rate_limit::{RateLimitDecision, RateLimiterStrategy};

#[async_trait]
pub trait RateLimiter: Send + Sync {
    async fn allow(
        &self,
        strategy: RateLimiterStrategy,
        key: &str,
    ) -> anyhow::Result<RateLimitDecision>;
}
