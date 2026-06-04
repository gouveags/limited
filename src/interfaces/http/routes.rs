use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use crate::{app::state::AppState, interfaces::http::handlers};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/counters/{key}", get(handlers::get_counter))
        .route(
            "/limiters/{strategy}/counters/{key}/increment",
            get(handlers::increment_counter).post(handlers::increment_counter),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
