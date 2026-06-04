use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;

use crate::{
    domain::rate_limit::{RateLimitDecision, RateLimiterStrategy},
    infrastructure::redis::RedisInfrastructure,
    ports::rate_limiter::RateLimiter,
};

const LIMIT: u64 = 500;
const WINDOW_SECONDS: u64 = 60;
const REFILL_PER_MILLISECOND: f64 = LIMIT as f64 / (WINDOW_SECONDS * 1000) as f64;

#[derive(Clone)]
pub struct RedisRateLimiter {
    redis: RedisInfrastructure,
}

impl RedisRateLimiter {
    pub fn new(redis: RedisInfrastructure) -> Self {
        Self { redis }
    }

    fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is before unix epoch")
            .as_millis() as u64
    }

    pub async fn allow_at(
        &self,
        strategy: RateLimiterStrategy,
        key: &str,
        now_ms: u64,
    ) -> anyhow::Result<RateLimitDecision> {
        match strategy {
            RateLimiterStrategy::FixedWindow => self.fixed_window(key, now_ms).await,
            RateLimiterStrategy::SlidingWindowLog => self.sliding_window_log(key, now_ms).await,
            RateLimiterStrategy::TokenBucket => self.token_bucket(key, now_ms).await,
            RateLimiterStrategy::LeakyBucket => self.leaky_bucket(key, now_ms).await,
        }
    }

    async fn fixed_window(&self, key: &str, now_ms: u64) -> anyhow::Result<RateLimitDecision> {
        let window = now_ms / (WINDOW_SECONDS * 1000);
        let redis_key = format!("rate_limit:fixed_window:{key}:{window}");
        let script = redis::Script::new(
            r#"
            local count = redis.call("INCR", KEYS[1])
            if count == 1 then
                redis.call("PEXPIRE", KEYS[1], ARGV[2])
            end

            if count > tonumber(ARGV[1]) then
                return {0, 0, tonumber(ARGV[2])}
            end

            return {1, tonumber(ARGV[1]) - count, 0}
            "#,
        );

        self.run_decision_script(script.key(redis_key).arg(LIMIT).arg(WINDOW_SECONDS * 1000))
            .await
    }

    async fn sliding_window_log(
        &self,
        key: &str,
        now_ms: u64,
    ) -> anyhow::Result<RateLimitDecision> {
        let redis_key = format!("rate_limit:sliding_window_log:{key}");
        let sequence_key = format!("rate_limit:sliding_window_log_sequence:{key}");
        let window_start = now_ms.saturating_sub(WINDOW_SECONDS * 1000);
        let script = redis::Script::new(
            r#"
            redis.call("ZREMRANGEBYSCORE", KEYS[1], 0, ARGV[2])
            local count = redis.call("ZCARD", KEYS[1])

            if count >= tonumber(ARGV[1]) then
                local oldest = redis.call("ZRANGE", KEYS[1], 0, 0, "WITHSCORES")
                local retry_after = tonumber(ARGV[4])
                if oldest[2] ~= nil then
                    retry_after = math.max(1, tonumber(oldest[2]) + tonumber(ARGV[4]) - tonumber(ARGV[3]))
                end
                return {0, 0, retry_after}
            end

            local sequence = redis.call("INCR", KEYS[2])
            redis.call("PEXPIRE", KEYS[2], ARGV[4])
            local member = tostring(ARGV[3]) .. ":" .. tostring(sequence)

            redis.call("ZADD", KEYS[1], ARGV[3], member)
            redis.call("PEXPIRE", KEYS[1], ARGV[4])
            return {1, tonumber(ARGV[1]) - count - 1, 0}
            "#,
        );

        self.run_decision_script(
            script
                .key(redis_key)
                .key(sequence_key)
                .arg(LIMIT)
                .arg(window_start)
                .arg(now_ms)
                .arg(WINDOW_SECONDS * 1000),
        )
        .await
    }

    async fn token_bucket(&self, key: &str, now_ms: u64) -> anyhow::Result<RateLimitDecision> {
        let redis_key = format!("rate_limit:token_bucket:{key}");
        let script = redis::Script::new(
            r#"
            local limit = tonumber(ARGV[1])
            local now = tonumber(ARGV[2])
            local refill_per_ms = tonumber(ARGV[3])
            local ttl = tonumber(ARGV[4])

            local state = redis.call("HMGET", KEYS[1], "tokens", "updated_at")
            local tokens = tonumber(state[1]) or limit
            local updated_at = tonumber(state[2]) or now
            local elapsed = math.max(0, now - updated_at)
            tokens = math.min(limit, tokens + (elapsed * refill_per_ms))

            if tokens < 1 then
                local retry_after = math.ceil((1 - tokens) / refill_per_ms)
                return {0, 0, retry_after}
            end

            tokens = tokens - 1
            redis.call("HSET", KEYS[1], "tokens", tokens, "updated_at", now)
            redis.call("PEXPIRE", KEYS[1], ttl)
            return {1, math.floor(tokens), 0}
            "#,
        );

        self.run_decision_script(
            script
                .key(redis_key)
                .arg(LIMIT)
                .arg(now_ms)
                .arg(REFILL_PER_MILLISECOND)
                .arg(WINDOW_SECONDS * 2000),
        )
        .await
    }

    async fn leaky_bucket(&self, key: &str, now_ms: u64) -> anyhow::Result<RateLimitDecision> {
        let redis_key = format!("rate_limit:leaky_bucket:{key}");
        let script = redis::Script::new(
            r#"
            local limit = tonumber(ARGV[1])
            local now = tonumber(ARGV[2])
            local leak_per_ms = tonumber(ARGV[3])
            local ttl = tonumber(ARGV[4])

            local state = redis.call("HMGET", KEYS[1], "level", "updated_at")
            local level = tonumber(state[1]) or 0
            local updated_at = tonumber(state[2]) or now
            local elapsed = math.max(0, now - updated_at)
            level = math.max(0, level - (elapsed * leak_per_ms))

            if level + 1 > limit then
                local retry_after = math.ceil((level + 1 - limit) / leak_per_ms)
                return {0, 0, retry_after}
            end

            level = level + 1
            redis.call("HSET", KEYS[1], "level", level, "updated_at", now)
            redis.call("PEXPIRE", KEYS[1], ttl)
            return {1, limit - math.ceil(level), 0}
            "#,
        );

        self.run_decision_script(
            script
                .key(redis_key)
                .arg(LIMIT)
                .arg(now_ms)
                .arg(REFILL_PER_MILLISECOND)
                .arg(WINDOW_SECONDS * 2000),
        )
        .await
    }

    async fn run_decision_script(
        &self,
        invocation: &mut redis::ScriptInvocation<'_>,
    ) -> anyhow::Result<RateLimitDecision> {
        let mut connection = self.redis.connection().await?;
        let result: (u8, u64, u64) = invocation.invoke_async(&mut connection).await?;

        let decision = if result.0 == 1 {
            RateLimitDecision::allowed(LIMIT, result.1)
        } else {
            RateLimitDecision::rejected(LIMIT, millis_to_seconds(result.2))
        };

        Ok(decision)
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn allow(
        &self,
        strategy: RateLimiterStrategy,
        key: &str,
    ) -> anyhow::Result<RateLimitDecision> {
        self.allow_at(strategy, key, Self::now_millis()).await
    }
}

fn millis_to_seconds(millis: u64) -> u64 {
    millis.div_ceil(1000).max(1)
}
