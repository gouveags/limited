use std::sync::Arc;

use thiserror::Error;

use crate::{
    domain::{
        counter::Counter,
        rate_limit::{RateLimitDecision, RateLimiterStrategy},
    },
    ports::{counter_repository::CounterRepository, rate_limiter::RateLimiter},
};

#[derive(Clone)]
pub struct IncrementCounterService {
    counter_repository: Arc<dyn CounterRepository>,
    rate_limiter: Arc<dyn RateLimiter>,
}

impl IncrementCounterService {
    pub fn new(
        counter_repository: Arc<dyn CounterRepository>,
        rate_limiter: Arc<dyn RateLimiter>,
    ) -> Self {
        Self {
            counter_repository,
            rate_limiter,
        }
    }

    pub async fn execute(
        &self,
        strategy: RateLimiterStrategy,
        key: &str,
    ) -> Result<IncrementCounterOutput, IncrementCounterError> {
        let rate_limit = self
            .rate_limiter
            .allow(strategy, key)
            .await
            .map_err(IncrementCounterError::Unexpected)?;

        if !rate_limit.allowed {
            return Err(IncrementCounterError::RateLimited(rate_limit));
        }

        let counter = self
            .counter_repository
            .increment(key)
            .await
            .map_err(IncrementCounterError::Unexpected)?;

        Ok(IncrementCounterOutput {
            counter,
            rate_limit,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementCounterOutput {
    pub counter: Counter,
    pub rate_limit: RateLimitDecision,
}

#[derive(Debug, Error)]
pub enum IncrementCounterError {
    #[error("request rate limited")]
    RateLimited(RateLimitDecision),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}
