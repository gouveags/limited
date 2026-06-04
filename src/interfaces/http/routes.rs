use axum::{Router, routing::get};
use tower_http::trace::TraceLayer;

use crate::{app::state::AppState, interfaces::http::handlers};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
