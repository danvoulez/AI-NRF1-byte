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
// Ghost routes — WBE lifecycle (BASE terrain)
//
// No database. Persistence is through the LedgerWriter trait.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateGhostReq {
    pub cid: String,
    pub did: String,
    pub wbe: serde_json::Value,
}

#[derive(Serialize)]
pub struct GhostResp {
    pub id: Uuid,
    pub url: String,
    pub cid: String,
    pub status: String,
}

#[derive(Deserialize)]
pub struct PromoteReq {
    pub receipt_cid: String,
}

#[derive(Deserialize)]
pub struct ExpireReq {
    pub cause: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:app/:tenant/ghosts", post(create_ghost))
        .route("/:app/:tenant/ghosts/:id/promote", post(promote_ghost))
        .route("/:app/:tenant/ghosts/:id/expire", post(expire_ghost))
}

async fn create_ghost(
    Path((app, tenant)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateGhostReq>,
) -> Result<Json<GhostResp>, (axum::http::StatusCode, String)> {
    use crate::middleware::rbac;

    let user_id =
        rbac::parse_user_id(&headers).map_err(|s| (s, "missing or invalid x-user-id".into()))?;

    let id = Uuid::now_v7();
    let base = format!(
        "{}/{}/{}/ghosts/{}.json",
        state.cfg.public_base.trim_end_matches('/'),
        app,
        tenant,
        id
    );
    let url = format!("{}#cid={}&did={}", base, req.cid, req.did);

    // --- Persist to ledger ---
    {
        use ubl_storage::ledger::{LedgerEntry, LedgerEvent};
        let entry = LedgerEntry::now(
            LedgerEvent::GhostCreated,
            &app,
            &tenant,
            Some(user_id),
            vec![],
            id,
            &req.cid,
            &req.did,
            None,
            req.wbe.clone(),
        );
        if let Err(e) = state.ledger.append(&entry).await {
            tracing::warn!("ledger append (ghost create) failed: {}", e);
        }
    }

    Ok(Json(GhostResp {
        id,
        url,
        cid: req.cid,
        status: "pending".into(),
    }))
}

async fn promote_ghost(
    Path((app, tenant, id)): Path<(String, String, Uuid)>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<PromoteReq>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    {
        use ubl_storage::ledger::{LedgerEntry, LedgerEvent};
        let entry = LedgerEntry::now(
            LedgerEvent::GhostPromoted,
            &app,
            &tenant,
            None,
            vec![],
            id,
            &req.receipt_cid,
            "",
            None,
            serde_json::json!({"receipt_cid": req.receipt_cid}),
        );
        if let Err(e) = state.ledger.append(&entry).await {
            tracing::warn!("ledger append (ghost promote) failed: {}", e);
        }
    }
    Ok(Json(
        serde_json::json!({"ok": true, "ghost_id": id, "receipt_cid": req.receipt_cid}),
    ))
}

async fn expire_ghost(
    Path((app, tenant, id)): Path<(String, String, Uuid)>,
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExpireReq>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    {
        use ubl_storage::ledger::{LedgerEntry, LedgerEvent};
        let entry = LedgerEntry::now(
            LedgerEvent::GhostExpired,
            &app,
            &tenant,
            None,
            vec![],
            id,
            "",
            "",
            None,
            serde_json::json!({"cause": req.cause}),
        );
        if let Err(e) = state.ledger.append(&entry).await {
            tracing::warn!("ledger append (ghost expire) failed: {}", e);
        }
    }
    Ok(Json(
        serde_json::json!({"ok": true, "ghost_id": id, "cause": req.cause}),
    ))
}
