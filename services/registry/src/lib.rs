pub mod middleware;
pub mod routes;
pub mod state;

use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct Health {
    status: &'static str,
    #[cfg(feature = "modules")]
    modules: bool,
}

#[derive(Serialize)]
struct Version {
    version: &'static str,
    git_sha: &'static str,
    build_ts: &'static str,
    #[cfg(feature = "modules")]
    modules: bool,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        #[cfg(feature = "modules")]
        modules: true,
    })
}

async fn version() -> Json<Version> {
    Json(Version {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("BUILD_GIT_SHA").unwrap_or("dev"),
        build_ts: option_env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
        #[cfg(feature = "modules")]
        modules: true,
    })
}

/// Build the full Axum router for the registry service.
/// Used by main.rs and integration tests.
pub fn build_router(state: Arc<state::AppState>) -> Router {
    let base = Router::new()
        .route("/health", get(health))
        .route("/healthz", get(health))
        .route("/readyz", get(health))
        .route("/version", get(version))
        .nest("/v1", routes::receipts::router())
        .nest("/v1", routes::ghosts::router())
        .with_state(state);

    // When compiled with --features modules, mount permit + pipeline routes
    #[cfg(feature = "modules")]
    let base = {
        let state_dir = std::env::var("STATE_DIR").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            format!("{home}/.ai-nrf1/state")
        });
        let (modules_state, permit_state) = routes::modules::init_modules_state(&state_dir);
        tracing::info!(state_dir = %state_dir, "modules layer enabled");
        base.merge(routes::modules::modules_router(modules_state, permit_state))
    };

    base
}
