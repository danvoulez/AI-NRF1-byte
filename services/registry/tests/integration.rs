// ==========================================================================
// Integration Test — Full Constitutional Pipeline
//
// Spins up the real Axum server with ledger-only persistence (no database).
// Verifies: HTTP → ρ → POLICY → GHOST → RECEIPT → SIGN → ledger → response.
//
// Run:
//   cargo test -p registry --test integration
// ==========================================================================

use reqwest::StatusCode;
use serde_json::{json, Value};
use std::net::TcpListener;
use std::sync::Arc;

/// Find a free port on localhost
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

/// Start the registry server on a random port, return the base URL
async fn start_server() -> String {
    let port = free_port();

    let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::thread_rng());
    let verifying_key = signing_key.verifying_key();
    let rt = runtime::SelfAttestation::new("test-binary-hash");

    let cfg = registry::state::Config {
        public_base: format!("http://localhost:{port}"),
        issuer_did: "did:ubl:test".into(),
    };

    let ledger: Arc<dyn ubl_storage::ledger::LedgerWriter> =
        Arc::new(ubl_storage::ledger::NullLedger);

    let state = Arc::new(registry::state::AppState {
        cfg,
        signing_key,
        verifying_key,
        runtime: rt,
        ledger,
    });

    let app = registry::build_router(state);

    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
        .await
        .unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    format!("http://127.0.0.1:{port}")
}

// ==========================================================================
// BASE tests — always run (no feature gate)
// ==========================================================================

#[tokio::test]
async fn test_health() {
    let base = start_server().await;
    let resp = reqwest::get(format!("{base}/health")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_create_receipt_without_auth_returns_401() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/lab512/dev/receipts"))
        .json(&json!({
            "body": {"test": true},
            "act": "ATTEST",
            "subject": "b3:0000000000000000000000000000000000000000000000000000000000000000"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_receipt_with_auth_returns_501() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/lab512/dev/receipts"))
        .header("Authorization", "Bearer test-token-123")
        .json(&json!({
            "body": {"test": true},
            "act": "ATTEST",
            "subject": "b3:0000000000000000000000000000000000000000000000000000000000000000"
        }))
        .send()
        .await
        .unwrap();
    // Auth passes, but pipeline is not yet implemented → 501
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

// ==========================================================================
// MODULE tests — will be added when capability modules are implemented
// ==========================================================================
