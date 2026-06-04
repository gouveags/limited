use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use limited::{
    app::state::AppState,
    domain::rate_limit::RateLimiterStrategy,
    infrastructure::redis::{RedisInfrastructure, rate_limiter::RedisRateLimiter},
    interfaces::http::{dto::IncrementCounterResponse, routes},
};
use tower::ServiceExt;

const LIMIT: usize = 500;

#[tokio::test]
async fn fixed_window_rejects_the_501st_request_inside_one_window() {
    let limiter = test_limiter().await;
    let key = test_key("fixed-limit");

    for request_number in 1..=LIMIT {
        let decision = limiter
            .allow_at(RateLimiterStrategy::FixedWindow, &key, 1_000)
            .await
            .unwrap();
        assert!(decision.allowed, "request {request_number} should pass");
    }

    let decision = limiter
        .allow_at(RateLimiterStrategy::FixedWindow, &key, 1_000)
        .await
        .unwrap();

    assert!(!decision.allowed);
    assert_eq!(decision.retry_after_seconds, Some(60));
}

#[tokio::test]
async fn fixed_window_allows_a_boundary_burst_across_two_adjacent_windows() {
    let limiter = test_limiter().await;
    let key = test_key("fixed-boundary");

    for request_number in 1..=LIMIT {
        let decision = limiter
            .allow_at(RateLimiterStrategy::FixedWindow, &key, 59_999)
            .await
            .unwrap();
        assert!(
            decision.allowed,
            "late-window request {request_number} should pass"
        );
    }

    for request_number in 1..=LIMIT {
        let decision = limiter
            .allow_at(RateLimiterStrategy::FixedWindow, &key, 60_000)
            .await
            .unwrap();
        assert!(
            decision.allowed,
            "next-window request {request_number} should also pass"
        );
    }
}

#[tokio::test]
async fn sliding_window_log_rejects_the_501st_request_even_with_same_millisecond_requests() {
    let limiter = test_limiter().await;
    let key = test_key("sliding-limit");

    for request_number in 1..=LIMIT {
        let decision = limiter
            .allow_at(RateLimiterStrategy::SlidingWindowLog, &key, 10_000)
            .await
            .unwrap();
        assert!(decision.allowed, "request {request_number} should pass");
    }

    let decision = limiter
        .allow_at(RateLimiterStrategy::SlidingWindowLog, &key, 10_000)
        .await
        .unwrap();

    assert!(!decision.allowed);
    assert_eq!(decision.retry_after_seconds, Some(60));
}

#[tokio::test]
async fn sliding_window_log_recovers_when_the_oldest_request_leaves_the_window() {
    let limiter = test_limiter().await;
    let key = test_key("sliding-recovery");

    for _ in 0..LIMIT {
        limiter
            .allow_at(RateLimiterStrategy::SlidingWindowLog, &key, 10_000)
            .await
            .unwrap();
    }

    let rejected = limiter
        .allow_at(RateLimiterStrategy::SlidingWindowLog, &key, 69_999)
        .await
        .unwrap();
    assert!(!rejected.allowed);

    let allowed = limiter
        .allow_at(RateLimiterStrategy::SlidingWindowLog, &key, 70_000)
        .await
        .unwrap();
    assert!(allowed.allowed);
}

#[tokio::test]
async fn token_bucket_allows_an_initial_burst_then_waits_for_refill() {
    let limiter = test_limiter().await;
    let key = test_key("token-burst");

    for request_number in 1..=LIMIT {
        let decision = limiter
            .allow_at(RateLimiterStrategy::TokenBucket, &key, 1_000)
            .await
            .unwrap();
        assert!(
            decision.allowed,
            "burst request {request_number} should pass"
        );
    }

    let rejected = limiter
        .allow_at(RateLimiterStrategy::TokenBucket, &key, 1_000)
        .await
        .unwrap();
    assert!(!rejected.allowed);
    assert_eq!(rejected.retry_after_seconds, Some(1));

    let refilled = limiter
        .allow_at(RateLimiterStrategy::TokenBucket, &key, 1_120)
        .await
        .unwrap();
    assert!(refilled.allowed);
}

#[tokio::test]
async fn leaky_bucket_allows_a_full_bucket_burst_then_waits_for_leakage() {
    let limiter = test_limiter().await;
    let key = test_key("leaky-burst");

    for request_number in 1..=LIMIT {
        let decision = limiter
            .allow_at(RateLimiterStrategy::LeakyBucket, &key, 1_000)
            .await
            .unwrap();
        assert!(
            decision.allowed,
            "burst request {request_number} should pass"
        );
    }

    let rejected = limiter
        .allow_at(RateLimiterStrategy::LeakyBucket, &key, 1_000)
        .await
        .unwrap();
    assert!(!rejected.allowed);
    assert_eq!(rejected.retry_after_seconds, Some(1));

    let leaked = limiter
        .allow_at(RateLimiterStrategy::LeakyBucket, &key, 1_120)
        .await
        .unwrap();
    assert!(leaked.allowed);
}

#[tokio::test]
async fn http_increment_endpoint_applies_the_selected_limiter_and_updates_the_counter() {
    let redis = test_redis().await;
    let app = routes::router(AppState::new(redis));
    let key = test_key("http");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/limiters/fixed-window/counters/{key}/increment"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let response: IncrementCounterResponse = serde_json::from_slice(&body).unwrap();

    assert_eq!(response.key, key);
    assert_eq!(response.value, 1);
    assert!(response.rate_limit.allowed);
    assert_eq!(response.rate_limit.limit, 500);
    assert_eq!(response.rate_limit.remaining, 499);
}

async fn test_limiter() -> RedisRateLimiter {
    RedisRateLimiter::new(test_redis().await)
}

async fn test_redis() -> RedisInfrastructure {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    RedisInfrastructure::connect(&redis_url).await.unwrap()
}

fn test_key(label: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("test:{label}:{now}")
}
