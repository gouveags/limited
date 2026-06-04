use anyhow::Context;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_address: String,
    pub redis_url: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            bind_address: std::env::var("BIND_ADDRESS")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            redis_url: std::env::var("REDIS_URL").context("REDIS_URL is required")?,
        })
    }
}
