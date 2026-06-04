use crate::infrastructure::redis::RedisInfrastructure;

#[derive(Clone)]
pub struct AppState {
    pub redis: RedisInfrastructure,
}

impl AppState {
    pub fn new(redis: RedisInfrastructure) -> Self {
        Self { redis }
    }
}
