pub mod middleware;
pub mod routes;
pub mod state;

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
        // K8s-style aliases (kept simple: if the process is serving, it's "ready").
        .route("/healthz", get(health))
        .route("/readyz", get(health))
        .nest("/v1", routes::receipts::router())
        .nest("/v1", routes::ghosts::router())
        .with_state(state)
}
