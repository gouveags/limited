use axum::{
    Json,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{
    application::increment_counter::IncrementCounterError,
    domain::rate_limit::RateLimitDecision,
    interfaces::http::dto::{ErrorResponse, RateLimitResponse},
};

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
    retry_after_seconds: Option<u64>,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body: ErrorResponse {
                error: message.into(),
            },
            retry_after_seconds: None,
        }
    }

    pub fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!(error = %error, "request failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: ErrorResponse {
                error: "internal server error".to_string(),
            },
            retry_after_seconds: None,
        }
    }

    fn rate_limited(decision: RateLimitDecision) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: ErrorResponse {
                error: "rate limit exceeded".to_string(),
            },
            retry_after_seconds: decision.retry_after_seconds,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();

        if let Some(retry_after_seconds) = self.retry_after_seconds {
            if let Ok(value) = HeaderValue::from_str(&retry_after_seconds.to_string()) {
                headers.insert(header::RETRY_AFTER, value);
            }
        }

        (self.status, headers, Json(self.body)).into_response()
    }
}

impl From<IncrementCounterError> for ApiError {
    fn from(error: IncrementCounterError) -> Self {
        match error {
            IncrementCounterError::RateLimited(decision) => Self::rate_limited(decision),
            IncrementCounterError::Unexpected(error) => Self::internal(error),
        }
    }
}

impl From<RateLimitDecision> for RateLimitResponse {
    fn from(decision: RateLimitDecision) -> Self {
        Self {
            allowed: decision.allowed,
            limit: decision.limit,
            remaining: decision.remaining,
            retry_after_seconds: decision.retry_after_seconds,
        }
    }
}
