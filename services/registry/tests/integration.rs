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

// ==========================================================================
// MODULE tests — only run when receipt-gateway is compiled in
// ==========================================================================

#[cfg(feature = "module-receipt-gateway")]
#[tokio::test]
async fn test_full_pipeline_creates_receipt() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/lab512/dev/receipts"))
        .header("x-user-id", "00000000-0000-0000-0000-000000000099")
        .json(&json!({
            "body": {"name": "integration-test", "value": 42},
            "act": "ATTEST",
            "subject": "b3:0000000000000000000000000000000000000000000000000000000000000000"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "pipeline failed: {}",
        resp.text().await.unwrap_or_default()
    );

    let resp = client
        .post(format!("{base}/v1/lab512/dev/receipts"))
        .header("x-user-id", "00000000-0000-0000-0000-000000000099")
        .json(&json!({
            "body": {"name": "integration-test-2", "value": 43},
            "act": "ATTEST",
            "subject": "b3:0000000000000000000000000000000000000000000000000000000000000000"
        }))
        .send()
        .await
        .unwrap();

    let receipt: Value = resp.json().await.unwrap();

    assert!(receipt["id"].is_string(), "must have id");
    assert!(
        receipt["receipt_cid"].as_str().unwrap().starts_with("b3:"),
        "receipt_cid must be b3:*"
    );
    assert!(
        receipt["ghost_cid"].as_str().unwrap().starts_with("b3:"),
        "ghost_cid must be b3:*"
    );
    assert_eq!(receipt["act"], "ATTEST");
    assert!(
        receipt["body_cid"].as_str().unwrap().starts_with("b3:"),
        "body_cid must be b3:*"
    );
    assert!(
        receipt["verifying_key_hex"].is_string(),
        "must return verifying key"
    );

    let cid = receipt["receipt_cid"].as_str().unwrap();
    let cid_hex = cid.strip_prefix("b3:").unwrap();
    assert_eq!(cid_hex.len(), 64, "CID hash must be 64 hex chars");
}

#[cfg(feature = "module-receipt-gateway")]
#[tokio::test]
async fn test_different_bodies_produce_different_cids() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let post =
        |body: Value| {
            let client = client.clone();
            let base = base.clone();
            async move {
                client.post(format!("{base}/v1/lab512/dev/receipts"))
                .header("x-user-id", "00000000-0000-0000-0000-000000000099")
                .json(&json!({
                    "body": body,
                    "act": "ATTEST",
                    "subject": "b3:0000000000000000000000000000000000000000000000000000000000000000"
                }))
                .send().await.unwrap()
                .json::<Value>().await.unwrap()
            }
        };

    let r1 = post(json!({"name": "alpha"})).await;
    let r2 = post(json!({"name": "beta"})).await;

    // Fractal invariant: different input → different CID
    assert_ne!(r1["receipt_cid"], r2["receipt_cid"]);
    assert_ne!(r1["body_cid"], r2["body_cid"]);
    assert_ne!(r1["id"], r2["id"]);
}
