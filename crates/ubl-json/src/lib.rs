use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UblJsonError {
    #[error("[Err.UblJson.Validation] {field} is empty. Hint: {hint}")]
    Validation {
        field: &'static str,
        hint: &'static str,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UblJsonV1 {
    pub space: String,
    pub version: String,
    pub id: String,
    pub app: String,
    pub tenant: String,
    pub subject: String,
    pub intent: String,
    #[serde(default)]
    pub scope: Option<String>,
    pub claims: Vec<String>,
    #[serde(default)]
    pub grounds: serde_json::Value,
    pub rules_ref: Vec<String>,
    #[serde(default)]
    pub decision_hint: Option<String>,
    #[serde(default)]
    pub confidence: Option<f64>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub meta: serde_json::Value,
}

impl UblJsonV1 {
    pub fn validate(&self) -> Result<(), UblJsonError> {
        if self.space.is_empty() {
            return Err(UblJsonError::Validation { field: "space", hint: "Set 'space' to the namespace for this document (e.g., 'compliance', 'identity')" });
        }
        if self.version.is_empty() {
            return Err(UblJsonError::Validation { field: "version", hint: "Set 'version' to the schema version (e.g., '1.0')" });
        }
        if self.id.is_empty() {
            return Err(UblJsonError::Validation { field: "id", hint: "Set 'id' to a unique document identifier (UUID or CID)" });
        }
        if self.app.is_empty() {
            return Err(UblJsonError::Validation { field: "app", hint: "Set 'app' to the application name that owns this document" });
        }
        if self.tenant.is_empty() {
            return Err(UblJsonError::Validation { field: "tenant", hint: "Set 'tenant' to the tenant slug (e.g., 'default', 'acme')" });
        }
        if self.subject.is_empty() {
            return Err(UblJsonError::Validation { field: "subject", hint: "Set 'subject' to the DID or identifier of the entity this document is about" });
        }
        if self.intent.is_empty() {
            return Err(UblJsonError::Validation { field: "intent", hint: "Set 'intent' to the action being requested (e.g., 'verify', 'approve', 'transact')" });
        }
        if self.claims.is_empty() {
            return Err(UblJsonError::Validation { field: "claims", hint: "Add at least one claim string to the 'claims' array" });
        }
        Ok(())
    }

    /// Placeholder: map to NRF logical value (delegates to nrf1-core value builder).
    pub fn to_nrf_value(&self) -> nrf_core::Value {
        use nrf_core::Value as V;
        // Minimal deterministic mapping (keys sorted in encoder)
        let mut map = std::collections::BTreeMap::new();
        map.insert("space".into(), V::String(self.space.clone()));
        map.insert("version".into(), V::String(self.version.clone()));
        map.insert("id".into(), V::String(self.id.clone()));
        map.insert("app".into(), V::String(self.app.clone()));
        map.insert("tenant".into(), V::String(self.tenant.clone()));
        map.insert("subject".into(), V::String(self.subject.clone()));
        map.insert("intent".into(), V::String(self.intent.clone()));
        if let Some(s) = &self.scope {
            map.insert("scope".into(), V::String(s.clone()));
        }
        map.insert(
            "claims".into(),
            V::Array(self.claims.iter().cloned().map(V::String).collect()),
        );
        // grounds/meta as JSON string blobs for now (can be refined)
        map.insert("grounds".into(), V::String(self.grounds.to_string()));
        map.insert(
            "rules_ref".into(),
            V::Array(self.rules_ref.iter().cloned().map(V::String).collect()),
        );
        if let Some(h) = &self.decision_hint {
            map.insert("decision_hint".into(), V::String(h.clone()));
        }
        if let Some(c) = self.confidence {
            map.insert(
                "confidence".into(),
                V::Int((c * 1_000_000.0).round() as i64),
            );
        }
        map.insert(
            "evidence".into(),
            V::Array(self.evidence.iter().cloned().map(V::String).collect()),
        );
        map.insert("meta".into(), V::String(self.meta.to_string()));
        V::Map(map)
    }
}
