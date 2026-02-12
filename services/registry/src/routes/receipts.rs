use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
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
    Router::new().route("/:app/:tenant/receipts", post(create_receipt))
}

// =========================================================================
// CREATE — stub (receipt pipeline not yet implemented via capability modules)
// =========================================================================

async fn create_receipt(
    Path((_app, _tenant)): Path<(String, String)>,
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(_req): Json<CreateReceiptReq>,
) -> Result<Json<ReceiptResp>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    // Auth gate: require Bearer token before anything else
    match headers.get(axum::http::header::AUTHORIZATION) {
        Some(v) if v.to_str().unwrap_or("").starts_with("Bearer ") => {}
        _ => {
            let err = ubl_error::UblError::missing_header(
                "Authorization",
                "Add 'Authorization: Bearer <token>' header to your request",
            );
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                Json(err.to_json()),
            ));
        }
    }

    let err = ubl_error::UblError {
        code: "Err.Receipt.NotImplemented".into(),
        message: "Receipt pipeline not yet implemented".into(),
        hint: "Use POST /api/v0/pipeline/run for module-based pipelines, or wait for receipt capability modules".into(),
        status: 501,
    };
    Err((
        axum::http::StatusCode::NOT_IMPLEMENTED,
        Json(err.to_json()),
    ))
}
