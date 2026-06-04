use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CounterResponse {
    pub key: String,
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncrementCounterResponse {
    pub key: String,
    pub value: u64,
    pub rate_limit: RateLimitResponse,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RateLimitResponse {
    pub allowed: bool,
    pub limit: u64,
    pub remaining: u64,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorResponse {
    pub error: String,
}
