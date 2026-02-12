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
    // Verify structured JSON error shape
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
    assert!(body["error"]["code"].as_str().unwrap().starts_with("Err."));
    assert!(body["error"]["message"].as_str().is_some());
    assert!(body["error"]["hint"].as_str().is_some());
    assert_eq!(body["error"]["status"], 401);
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
    // Verify structured JSON error shape
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"]["code"], "Err.Receipt.NotImplemented");
    assert!(body["error"]["hint"].as_str().unwrap().len() > 0);
    assert_eq!(body["error"]["status"], 501);
}

// ==========================================================================
// Error shape tests — verify all error responses are structured JSON
// ==========================================================================

#[tokio::test]
async fn test_all_errors_have_canonical_shape() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    // 401 — missing auth
    let resp = client
        .post(format!("{base}/v1/lab512/dev/receipts"))
        .json(&json!({"body": {}, "act": "ATTEST", "subject": "b3:00"}))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    verify_error_shape(&body, 401);

    // 501 — not implemented (with auth)
    let resp = client
        .post(format!("{base}/v1/lab512/dev/receipts"))
        .header("Authorization", "Bearer tok")
        .json(&json!({"body": {}, "act": "ATTEST", "subject": "b3:00"}))
        .send()
        .await
        .unwrap();
    let body: Value = resp.json().await.unwrap();
    verify_error_shape(&body, 501);
}

fn verify_error_shape(body: &Value, expected_status: u16) {
    assert_eq!(body["ok"], false, "error response must have ok: false");
    let err = &body["error"];
    let code = err["code"].as_str().expect("error.code must be a string");
    assert!(code.starts_with("Err."), "code must start with 'Err.': {code}");
    assert!(err["message"].as_str().is_some(), "error.message must be a string");
    assert!(err["hint"].as_str().is_some(), "error.hint must be a string");
    assert!(!err["hint"].as_str().unwrap().is_empty(), "error.hint must not be empty");
    assert_eq!(err["status"], expected_status, "error.status must match HTTP status");
}
