use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// ---------------------------------------------------------------------------
// Registry Service — BASE chassis
//
// This is the infrastructure that any product mounts on.
// It provides: health, receipt CRUD, ghost lifecycle, key resolution.
// Modules add policy engines, intake adapters, and enrichments on top.
//
// Same pipeline: INPUT → CANON → POLICY GATE → RUNTIME → RECEIPT → URL
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "registry=info,axum=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = registry::state::AppState::new().await?;
    let app = registry::build_router(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
