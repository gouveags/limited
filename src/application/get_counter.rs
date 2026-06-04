use std::sync::Arc;

use crate::{domain::counter::Counter, ports::counter_repository::CounterRepository};

#[derive(Clone)]
pub struct GetCounterService {
    counter_repository: Arc<dyn CounterRepository>,
}

impl GetCounterService {
    pub fn new(counter_repository: Arc<dyn CounterRepository>) -> Self {
        Self { counter_repository }
    }

    pub async fn execute(&self, key: &str) -> anyhow::Result<Counter> {
        self.counter_repository.get(key).await
    }
}
