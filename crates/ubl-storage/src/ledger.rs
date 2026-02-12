use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Ledger — BASE storage primitive (append-only audit trail)
//
// This trait defines WHAT an audit ledger does. Modules define HOW:
//   - ledger-ndjson: local filesystem NDJSON files (default)
//   - ledger-s3:     S3-backed NDJSON (future)
//
// The BASE ships NullLedger as fallback when no ledger module is compiled in.
//
// Layout convention (for file-based implementations):
//   {base_dir}/{app}/{tenant}/receipts.ndjson
//   {base_dir}/{app}/{tenant}/ghosts.ndjson
//
// Each line is a self-contained JSON object with full RBAC context.
// Files are append-only. No line is ever modified or deleted.
// This is the audit trail that survives database failures.
// ---------------------------------------------------------------------------

/// What kind of event is being recorded
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEvent {
    ReceiptCreated,
    GhostCreated,
    GhostPromoted,
    GhostExpired,
    PipelineExecuted,
}

/// A single append-only ledger entry with full RBAC context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub ts: String, // RFC-3339 UTC
    pub event: LedgerEvent,
    pub app: String,                // app slug
    pub tenant: String,             // tenant slug
    pub user_id: Option<Uuid>,      // who performed the action
    pub roles: Vec<String>,         // RBAC roles at time of action
    pub entity_id: Uuid,            // receipt or ghost UUID
    pub cid: String,                // content-addressed ID
    pub did: String,                // issuer or actor DID
    pub decision: Option<String>,   // ALLOW | DENY | REQUIRE | GHOST
    pub payload: serde_json::Value, // full signed object
}

impl LedgerEntry {
    #[allow(clippy::too_many_arguments)]
    pub fn now(
        event: LedgerEvent,
        app: &str,
        tenant: &str,
        user_id: Option<Uuid>,
        roles: Vec<String>,
        entity_id: Uuid,
        cid: &str,
        did: &str,
        decision: Option<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            event,
            app: app.to_string(),
            tenant: tenant.to_string(),
            user_id,
            roles,
            entity_id,
            cid: cid.to_string(),
            did: did.to_string(),
            decision,
            payload,
        }
    }

    /// Which NDJSON stream this entry belongs to
    pub fn stream_name(&self) -> &'static str {
        match self.event {
            LedgerEvent::ReceiptCreated => "receipts",
            LedgerEvent::GhostCreated | LedgerEvent::GhostPromoted | LedgerEvent::GhostExpired => {
                "ghosts"
            }
            LedgerEvent::PipelineExecuted => "executions",
        }
    }
}

// =========================================================================
// Trait — the BASE socket
// =========================================================================

#[derive(Debug)]
pub enum LedgerError {
    Io(String),
    Serialization(String),
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LedgerError::Io(e) => write!(f, "ledger I/O: {e}"),
            LedgerError::Serialization(e) => write!(f, "ledger serialize: {e}"),
        }
    }
}

impl From<std::io::Error> for LedgerError {
    fn from(e: std::io::Error) -> Self {
        LedgerError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for LedgerError {
    fn from(e: serde_json::Error) -> Self {
        LedgerError::Serialization(e.to_string())
    }
}

/// Append-only ledger writer. BASE terrain.
///
/// Modules implement this trait to provide storage backends.
/// The BASE ships NullLedger as fallback.
#[async_trait::async_trait]
pub trait LedgerWriter: Send + Sync {
    async fn append(&self, entry: &LedgerEntry) -> Result<(), LedgerError>;
}

// =========================================================================
// NullLedger — BASE fallback (no-op, for when no ledger module is compiled)
// =========================================================================

pub struct NullLedger;

#[async_trait::async_trait]
impl LedgerWriter for NullLedger {
    async fn append(&self, _entry: &LedgerEntry) -> Result<(), LedgerError> {
        Ok(())
    }
}
