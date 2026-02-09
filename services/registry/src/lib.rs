pub mod state;
pub mod middleware;
pub mod routes;

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

/// Build the full Axum router for the registry service.
/// Used by main.rs and integration tests.
pub fn build_router(state: Arc<state::AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .nest("/v1", routes::receipts::router())
        .nest("/v1", routes::ghosts::router())
        .with_state(state)
}
