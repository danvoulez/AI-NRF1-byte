use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Policy Gate — BASE terrain
//
// This is the SOCKET. Modules plug policy families into it.
// The 5 families (existence, compliance, threshold, provenance, authorization)
// are MODULE phase. This crate defines the trait and the wire types.
//
// Same pipeline position: INPUT → CANON → **POLICY GATE** → RUNTIME → RECEIPT
// ---------------------------------------------------------------------------

/// The 5 policy families. A module declares which family it belongs to.
/// This is metadata for routing and composition — the engine doesn't branch on it.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum PolicyFamily {
    Existence,     // is it well-formed? required fields present?
    Compliance,    // does it meet regulatory rules?
    Threshold,     // is a numeric value within bounds?
    Provenance,    // was it built/signed/attested correctly?
    Authorization, // can this actor do this thing to this resource?
}

/// What the gate receives. Every field is a CID or a reference — no raw data.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalRequest {
    pub policy_id: String,        // e.g. "pack-compliance/eu-ai-act@1"
    pub context_cid: String,      // b3:<hex> of the input context
    pub input: serde_json::Value, // the structured input (JSON view)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pipeline_prev: Vec<String>, // CIDs of prior act receipts
}

/// The 4 possible decisions.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    Require, // needs consent (k-of-n) before proceeding
    Ghost,   // write-before-execute: recorded but not effectuated
}

impl Decision {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Allow => "ALLOW",
            Self::Deny => "DENY",
            Self::Require => "REQUIRE",
            Self::Ghost => "GHOST",
        }
    }
}

/// What the gate returns.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalResponse {
    pub decision: Decision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_hint: Option<String>, // human-readable explanation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_cid: Option<String>, // b3:<hex> of ReasoningBit (if certified runtime ran)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules_fired: Vec<String>, // which policy units matched
}

/// The trait. Any module implements this to plug a policy family in.
pub trait PolicyEngine: Send + Sync {
    fn evaluate(&self, req: &EvalRequest) -> anyhow::Result<EvalResponse>;

    /// Which family this engine belongs to. Used for routing and composition.
    fn family(&self) -> PolicyFamily;
}
