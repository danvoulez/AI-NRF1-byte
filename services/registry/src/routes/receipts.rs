use axum::{extract::{State, Path}, routing::post, Json, Router};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

// ---------------------------------------------------------------------------
// Receipt routes — BASE chassis + optional MODULE pipeline
//
// No database. Persistence is through the LedgerWriter trait.
// The ledger is the append-only audit trail and the source of truth.
//
// Without the module:  POST /receipts returns 501 Not Implemented.
// With the module:     POST /receipts runs INPUT → ρ → POLICY → GHOST → SIGN → ledger.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateReceiptReq {
    pub body: serde_json::Value,
    pub act: String,
    pub subject: String,
    pub policy_id: Option<String>,
    #[serde(default)]
    pub pipeline_prev: Vec<String>,
}

#[derive(Serialize)]
pub struct ReceiptResp {
    pub id: Uuid,
    pub receipt_cid: String,
    pub ghost_cid: String,
    pub url: String,
    pub decision: String,
    pub act: String,
    pub body_cid: String,
    pub reasoning_hint: Option<String>,
    pub verifying_key_hex: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:app/:tenant/receipts", post(create_receipt))
}

// =========================================================================
// CREATE — requires module-receipt-gateway feature
// =========================================================================

#[cfg(feature = "module-receipt-gateway")]
async fn create_receipt(
    Path((app, tenant)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateReceiptReq>,
) -> Result<Json<ReceiptResp>, (axum::http::StatusCode, String)> {
    use crate::middleware::rbac;
    use runtime::RuntimeAttestation;

    let user_id = rbac::parse_user_id(&headers)
        .map_err(|s| (s, "missing or invalid x-user-id".into()))?;

    // --- Convert JSON body to NRF Value ---
    let body = json_to_nrf(&req.body)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, format!("body conversion: {}", e)))?;

    // --- Runtime attestation (Article VIII) ---
    let attest_req = runtime::AttestationRequest {
        input_cid: "pending".into(),
        act: req.act.clone(),
        policy_id: req.policy_id.clone(),
    };
    let attest_resp = state.runtime.attest(&attest_req)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("runtime attestation: {}", e)))?;

    let rt_info = receipt::RuntimeInfo {
        name: attest_resp.info.name,
        version: attest_resp.info.version,
        binary_sha256: attest_resp.info.binary_sha256,
        hal_ref: attest_resp.info.hal_ref,
        env: attest_resp.info.env,
        certs: attest_resp.info.certs,
    };

    let id = Uuid::now_v7();
    let url_base = format!("{}/{}/{}/receipts/{}.json",
        state.cfg.public_base.trim_end_matches('/'), app, tenant, id);

    let nonce: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();

    // --- Run the full constitutional pipeline ---
    let gateway_req = receipt_gateway::GatewayRequest {
        issuer_did: state.cfg.issuer_did.clone(),
        act: req.act.clone(),
        subject: req.subject.clone(),
        body,
        policy_id: req.policy_id.clone(),
        pipeline_prev: req.pipeline_prev.clone(),
        url_base,
        nonce,
    };

    let policy = receipt_gateway::ExistencePolicy;
    let result = receipt_gateway::execute(gateway_req, &policy, &state.signing_key, rt_info)
        .map_err(|e| (axum::http::StatusCode::UNPROCESSABLE_ENTITY, format!("{}", e)))?;

    let receipt_json = serde_json::to_value(&result.receipt)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let decision_str = result.receipt.decision.clone().unwrap_or_default();
    let ghost_json = serde_json::to_value(&result.ghost)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let ghost_id = Uuid::now_v7();

    // --- Persist to ledger (the only persistence) ---
    {
        use ubl_storage::ledger::{LedgerEntry, LedgerEvent};

        let receipt_entry = LedgerEntry::now(
            LedgerEvent::ReceiptCreated,
            &app, &tenant,
            Some(user_id), vec![],
            id, &result.receipt.receipt_cid, &result.receipt.issuer_did,
            Some(decision_str.clone()), receipt_json,
        );
        if let Err(e) = state.ledger.append(&receipt_entry).await {
            tracing::warn!("ledger append (receipt) failed: {}", e);
        }

        let ghost_entry = LedgerEntry::now(
            LedgerEvent::GhostCreated,
            &app, &tenant,
            Some(user_id), vec![],
            ghost_id, &result.ghost.ghost_cid, &result.ghost.wbe.who,
            None, ghost_json,
        );
        if let Err(e) = state.ledger.append(&ghost_entry).await {
            tracing::warn!("ledger append (ghost) failed: {}", e);
        }
    }

    let vk_hex = hex::encode(state.verifying_key.as_bytes());

    Ok(Json(ReceiptResp {
        id,
        receipt_cid: result.receipt.receipt_cid,
        ghost_cid: result.ghost.ghost_cid,
        url: result.receipt.url,
        decision: decision_str,
        act: result.receipt.act,
        body_cid: result.receipt.body_cid,
        reasoning_hint: result.reasoning_hint,
        verifying_key_hex: vk_hex,
    }))
}

#[cfg(not(feature = "module-receipt-gateway"))]
async fn create_receipt(
    Path((_app, _tenant)): Path<(String, String)>,
    State(_state): State<Arc<AppState>>,
    _headers: axum::http::HeaderMap,
    Json(_req): Json<CreateReceiptReq>,
) -> Result<Json<ReceiptResp>, (axum::http::StatusCode, String)> {
    Err((
        axum::http::StatusCode::NOT_IMPLEMENTED,
        "receipt-gateway module not compiled in. Build with: cargo build -p registry --features module-receipt-gateway".into(),
    ))
}

// =========================================================================
// Helpers
// =========================================================================

#[cfg(feature = "module-receipt-gateway")]
fn json_to_nrf(v: &serde_json::Value) -> Result<nrf_core::Value, String> {
    match v {
        serde_json::Value::Null => Ok(nrf_core::Value::Null),
        serde_json::Value::Bool(b) => Ok(nrf_core::Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(nrf_core::Value::Int(i))
            } else {
                Err("floats are not allowed in NRF (Article III)".into())
            }
        }
        serde_json::Value::String(s) => {
            if let Some(hex_str) = s.strip_prefix("$bytes:") {
                let bytes = hex::decode(hex_str).map_err(|e| format!("bad hex in $bytes: {}", e))?;
                Ok(nrf_core::Value::Bytes(bytes))
            } else {
                Ok(nrf_core::Value::String(s.clone()))
            }
        }
        serde_json::Value::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_to_nrf).collect();
            Ok(nrf_core::Value::Array(items?))
        }
        serde_json::Value::Object(obj) => {
            let mut m = std::collections::BTreeMap::new();
            for (k, v) in obj {
                m.insert(k.clone(), json_to_nrf(v)?);
            }
            Ok(nrf_core::Value::Map(m))
        }
    }
}
