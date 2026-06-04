use std::sync::Arc;

use crate::{
    application::{get_counter::GetCounterService, increment_counter::IncrementCounterService},
    infrastructure::redis::{
        RedisInfrastructure, counter_repository::RedisCounterRepository,
        rate_limiter::RedisRateLimiter,
    },
    ports::{counter_repository::CounterRepository, rate_limiter::RateLimiter},
};

#[derive(Clone)]
pub struct AppState {
    pub increment_counter: IncrementCounterService,
    pub get_counter: GetCounterService,
}

impl AppState {
    pub fn new(redis: RedisInfrastructure) -> Self {
        let counter_repository: Arc<dyn CounterRepository> =
            Arc::new(RedisCounterRepository::new(redis.clone()));
        let rate_limiter: Arc<dyn RateLimiter> = Arc::new(RedisRateLimiter::new(redis));

        Self {
            increment_counter: IncrementCounterService::new(
                counter_repository.clone(),
                rate_limiter,
            ),
            get_counter: GetCounterService::new(counter_repository),
        }
    }
}
