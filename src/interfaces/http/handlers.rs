use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
};

use crate::{
    app::state::AppState,
    domain::rate_limit::RateLimiterStrategy,
    interfaces::http::{
        dto::{CounterResponse, IncrementCounterResponse},
        error::ApiError,
    },
};

pub async fn health() -> &'static str {
    "ok"
}

pub async fn increment_counter(
    State(state): State<AppState>,
    Path((strategy, key)): Path<(String, String)>,
) -> Result<Json<IncrementCounterResponse>, ApiError> {
    let strategy = RateLimiterStrategy::from_str(&strategy)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;

    let output = state.increment_counter.execute(strategy, &key).await?;

    Ok(Json(IncrementCounterResponse {
        key: output.counter.key,
        value: output.counter.value,
        rate_limit: output.rate_limit.into(),
    }))
}

pub async fn get_counter(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<CounterResponse>, ApiError> {
    let counter = state
        .get_counter
        .execute(&key)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(CounterResponse {
        key: counter.key,
        value: counter.value,
    }))
}
