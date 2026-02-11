//! Minimal HTTP server for permit (consent) operations.
//!
//! Gated behind `feature = "server"`. Provides three endpoints:
//!   - `POST /permit/:tenant/:ticket_id/approve`
//!   - `POST /permit/:tenant/:ticket_id/deny`
//!   - `GET  /permit/:tenant/:ticket_id`
//!
//! Start with `permit_router()` and mount into an axum app.

#[cfg(feature = "server")]
pub use server::*;

#[cfg(feature = "server")]
mod server {
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
        Json, Router,
    };
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    use crate::adapters::permit::{PermitOutcome, PermitStore, TicketStatus};

    /// Shared state for the permit HTTP server.
    pub struct PermitState {
        pub store: PermitStore,
    }

    /// Request body for approve/deny.
    #[derive(Deserialize)]
    pub struct ApproveRequest {
        pub role: String,
        #[serde(default)]
        pub sig: Option<String>,
    }

    #[derive(Deserialize)]
    pub struct DenyRequest {
        pub role: String,
    }

    /// Response body.
    #[derive(Serialize)]
    pub struct PermitResponse {
        pub status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub approvals: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub needed: Option<u8>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub message: Option<String>,
    }

    /// Build the permit router. Mount at any prefix you like.
    ///
    /// ```ignore
    /// let state = Arc::new(PermitState { store: PermitStore::new("/tmp/state") });
    /// let app = permit_router(state);
    /// // axum::serve(listener, app).await
    /// ```
    pub fn permit_router(state: Arc<PermitState>) -> Router {
        Router::new()
            .route("/permit/{tenant}/{ticket_id}", get(get_ticket))
            .route("/permit/{tenant}/{ticket_id}/approve", post(approve_ticket))
            .route("/permit/{tenant}/{ticket_id}/deny", post(deny_ticket))
            .with_state(state)
    }

    fn now_nanos() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64
    }

    async fn get_ticket(
        State(state): State<Arc<PermitState>>,
        Path((tenant, ticket_id)): Path<(String, String)>,
    ) -> impl IntoResponse {
        match state.store.get(&tenant, &ticket_id) {
            Ok(Some(ticket)) => {
                (StatusCode::OK, Json(serde_json::to_value(&ticket).unwrap())).into_response()
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "ticket not found"})),
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response(),
        }
    }

    async fn approve_ticket(
        State(state): State<Arc<PermitState>>,
        Path((tenant, ticket_id)): Path<(String, String)>,
        Json(body): Json<ApproveRequest>,
    ) -> impl IntoResponse {
        let now = now_nanos();
        match state
            .store
            .approve(&tenant, &ticket_id, &body.role, now, body.sig)
        {
            Ok(PermitOutcome::Pending { approvals, needed }) => (
                StatusCode::OK,
                Json(PermitResponse {
                    status: "PENDING".into(),
                    approvals: Some(approvals),
                    needed: Some(needed),
                    message: None,
                }),
            )
                .into_response(),
            Ok(PermitOutcome::Closed(s)) => {
                let code = match s {
                    TicketStatus::Allow => StatusCode::OK,
                    TicketStatus::Expired => StatusCode::GONE,
                    _ => StatusCode::OK,
                };
                (
                    code,
                    Json(PermitResponse {
                        status: format!("{s:?}").to_uppercase(),
                        approvals: None,
                        needed: None,
                        message: None,
                    }),
                )
                    .into_response()
            }
            Ok(PermitOutcome::Rejected(reason)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PermitResponse {
                    status: "REJECTED".into(),
                    approvals: None,
                    needed: None,
                    message: Some(reason),
                }),
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response(),
        }
    }

    async fn deny_ticket(
        State(state): State<Arc<PermitState>>,
        Path((tenant, ticket_id)): Path<(String, String)>,
        Json(body): Json<DenyRequest>,
    ) -> impl IntoResponse {
        let now = now_nanos();
        match state.store.deny(&tenant, &ticket_id, &body.role, now) {
            Ok(PermitOutcome::Closed(s)) => (
                StatusCode::OK,
                Json(PermitResponse {
                    status: format!("{s:?}").to_uppercase(),
                    approvals: None,
                    needed: None,
                    message: None,
                }),
            )
                .into_response(),
            Ok(PermitOutcome::Rejected(reason)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(PermitResponse {
                    status: "REJECTED".into(),
                    approvals: None,
                    needed: None,
                    message: Some(reason),
                }),
            )
                .into_response(),
            Ok(other) => (
                StatusCode::OK,
                Json(serde_json::json!({"status": format!("{other:?}")})),
            )
                .into_response(),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
                .into_response(),
        }
    }
}
