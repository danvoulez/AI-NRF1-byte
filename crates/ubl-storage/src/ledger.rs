use nrf_core::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

    /// Convert to ρ-canonical NRF Value.
    ///
    /// This is the blessed path: struct → nrf_core::Value → ρ-normalize.
    /// The resulting Value has deterministic key order (BTreeMap),
    /// NFC-normalized strings, and null values stripped.
    pub fn to_canonical_value(&self) -> Result<Value, LedgerError> {
        let mut m = BTreeMap::new();

        m.insert("ts".into(), Value::String(self.ts.clone()));
        m.insert("event".into(), Value::String(event_to_string(&self.event)));
        m.insert("app".into(), Value::String(self.app.clone()));
        m.insert("tenant".into(), Value::String(self.tenant.clone()));

        if let Some(uid) = &self.user_id {
            m.insert("user_id".into(), Value::String(uid.to_string()));
        }
        // else: ρ rule 5 — absence, not null

        if !self.roles.is_empty() {
            let roles: Vec<Value> = self.roles.iter()
                .map(|r| Value::String(r.clone()))
                .collect();
            m.insert("roles".into(), Value::Array(roles));
        }

        m.insert("entity_id".into(), Value::String(self.entity_id.to_string()));
        m.insert("cid".into(), Value::String(self.cid.clone()));
        m.insert("did".into(), Value::String(self.did.clone()));

        if let Some(d) = &self.decision {
            m.insert("decision".into(), Value::String(d.clone()));
        }

        // payload: convert serde_json::Value → nrf_core::Value via ubl_json_view
        let nrf_payload = ubl_json_view::from_json(&self.payload)
            .map_err(|e| LedgerError::Serialization(format!("payload→NRF: {e}")))?;
        m.insert("payload".into(), nrf_payload);

        // ρ-normalize: NFC strings, strip nulls, sort keys
        nrf_core::rho::normalize(&Value::Map(m))
            .map_err(|e| LedgerError::Serialization(format!("ρ-normalize: {e}")))
    }

    /// Serialize to canonical JSON string (one NDJSON line).
    ///
    /// Path: struct → nrf_core::Value → ρ-normalize → ubl_json_view::to_json → JSON string.
    /// This guarantees deterministic output: same entry → same bytes → same hash.
    pub fn to_canonical_json(&self) -> Result<String, LedgerError> {
        let canonical = self.to_canonical_value()?;
        let json = ubl_json_view::to_json(&canonical);
        serde_json::to_string(&json).map_err(|e| LedgerError::Serialization(e.to_string()))
    }

    /// Deserialize from a canonical JSON line back into a LedgerEntry.
    ///
    /// Reads the canonical JSON, converts via from_json → NRF Value,
    /// then extracts fields. This is the inverse of to_canonical_json.
    pub fn from_canonical_json(line: &str) -> Result<Self, LedgerError> {
        // Parse JSON
        let json: serde_json::Value = serde_json::from_str(line)?;

        // Validate it round-trips through canonical NRF
        let _nrf = ubl_json_view::from_json(&json)
            .map_err(|e| LedgerError::Serialization(format!("from_json: {e}")))?;

        // Extract fields from the JSON object
        let obj = json.as_object()
            .ok_or_else(|| LedgerError::Serialization("expected JSON object".into()))?;

        let ts = obj.get("ts")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let event = obj.get("event")
            .and_then(|v| v.as_str())
            .map(string_to_event)
            .unwrap_or(LedgerEvent::PipelineExecuted);

        let app = obj.get("app")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let tenant = obj.get("tenant")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let user_id = obj.get("user_id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Uuid>().ok());

        let roles: Vec<String> = obj.get("roles")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let entity_id = obj.get("entity_id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Uuid>().ok())
            .unwrap_or(Uuid::nil());

        let cid = obj.get("cid")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let did = obj.get("did")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let decision = obj.get("decision")
            .and_then(|v| v.as_str())
            .map(String::from);

        let payload = obj.get("payload")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(Self {
            ts, event, app, tenant, user_id, roles,
            entity_id, cid, did, decision, payload,
        })
    }
}

fn event_to_string(e: &LedgerEvent) -> String {
    match e {
        LedgerEvent::ReceiptCreated => "receipt_created",
        LedgerEvent::GhostCreated => "ghost_created",
        LedgerEvent::GhostPromoted => "ghost_promoted",
        LedgerEvent::GhostExpired => "ghost_expired",
        LedgerEvent::PipelineExecuted => "pipeline_executed",
    }.to_string()
}

fn string_to_event(s: &str) -> LedgerEvent {
    match s {
        "receipt_created" => LedgerEvent::ReceiptCreated,
        "ghost_created" => LedgerEvent::GhostCreated,
        "ghost_promoted" => LedgerEvent::GhostPromoted,
        "ghost_expired" => LedgerEvent::GhostExpired,
        _ => LedgerEvent::PipelineExecuted,
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
