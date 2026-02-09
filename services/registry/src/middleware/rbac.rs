use uuid::Uuid;

// ---------------------------------------------------------------------------
// RBAC — BASE terrain
//
// Identity extraction from request headers. The BASE defines the socket.
// No database. Role resolution is a MODULE concern (future).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct AuthCtx {
    pub user_id: Uuid,
    pub roles: Vec<String>,
}

pub fn require_any(ctx: &AuthCtx, allowed: &[&str]) -> bool {
    ctx.roles.iter().any(|r| allowed.contains(&r.as_str()))
}

/// Extract user_id from x-user-id header.
pub fn parse_user_id(headers: &axum::http::HeaderMap) -> Result<Uuid, axum::http::StatusCode> {
    let hdr = headers.get("x-user-id").ok_or(axum::http::StatusCode::UNAUTHORIZED)?;
    let s = hdr.to_str().map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;
    Uuid::parse_str(s).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)
}
