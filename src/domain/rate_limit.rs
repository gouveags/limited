use std::{fmt, str::FromStr};

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimiterStrategy {
    FixedWindow,
    SlidingWindowLog,
    TokenBucket,
    LeakyBucket,
}

impl RateLimiterStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FixedWindow => "fixed-window",
            Self::SlidingWindowLog => "sliding-window-log",
            Self::TokenBucket => "token-bucket",
            Self::LeakyBucket => "leaky-bucket",
        }
    }
}

impl fmt::Display for RateLimiterStrategy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for RateLimiterStrategy {
    type Err = ParseRateLimiterStrategyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "fixed-window" => Ok(Self::FixedWindow),
            "sliding-window-log" => Ok(Self::SlidingWindowLog),
            "token-bucket" => Ok(Self::TokenBucket),
            "leaky-bucket" => Ok(Self::LeakyBucket),
            _ => Err(ParseRateLimiterStrategyError {
                value: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Error)]
#[error("unknown rate limiter strategy `{value}`")]
pub struct ParseRateLimiterStrategyError {
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub limit: u64,
    pub remaining: u64,
    pub retry_after_seconds: Option<u64>,
}

impl RateLimitDecision {
    pub fn allowed(limit: u64, remaining: u64) -> Self {
        Self {
            allowed: true,
            limit,
            remaining,
            retry_after_seconds: None,
        }
    }

    pub fn rejected(limit: u64, retry_after_seconds: u64) -> Self {
        Self {
            allowed: false,
            limit,
            remaining: 0,
            retry_after_seconds: Some(retry_after_seconds.max(1)),
        }
    }
}
