use serde::{Serialize, Deserialize};

// ---------------------------------------------------------------------------
// Act Templates — the 3 basis vectors (BASE terrain)
//
// Every product is a composition of these 3 acts.
// MODULE phase composes them into products via pipeline_prev.
//
// ATTEST:   "this thing exists and has these properties"
// EVALUATE: "given these rules, this passes or fails"
// TRANSACT: "two parties exchanged something, here's proof"
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Act {
    #[serde(rename = "ATTEST")]
    Attest,
    #[serde(rename = "EVALUATE")]
    Evaluate,
    #[serde(rename = "TRANSACT")]
    Transact,
}

impl Act {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Attest => "ATTEST",
            Self::Evaluate => "EVALUATE",
            Self::Transact => "TRANSACT",
        }
    }
}

// ---------------------------------------------------------------------------
// Shared context — every act request carries this
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Context {
    pub app: String,
    pub tenant: String,
    pub actor_did: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pipeline_prev: Vec<String>,     // CIDs of prior act receipts
}

// ---------------------------------------------------------------------------
// ATTEST — "this thing exists and has these properties"
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subject {
    pub kind: String,                   // e.g. "model_card", "sbom", "document", "artifact"
    pub id: String,                     // external identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,           // b3:<hex> if already hashed
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Evidence {
    pub kind: String,                   // e.g. "benchmark", "scan", "certificate"
    pub cid: String,                    // b3:<hex>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttestRequest {
    pub subject: Subject,
    pub properties: serde_json::Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    pub context: Context,
}

// ---------------------------------------------------------------------------
// EVALUATE — "given these rules, this passes or fails"
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvaluateRequest {
    pub subject: Subject,
    pub rules_ref: Vec<String>,         // policy pack references
    pub facts: serde_json::Value,       // the data to evaluate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipeline_prev: Option<String>,  // CID of prior act receipt (e.g. ATTEST)
    pub context: Context,
}

// ---------------------------------------------------------------------------
// TRANSACT — "two parties exchanged something, here's proof"
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Party {
    pub did: String,
    pub role: String,                   // e.g. "sender", "receiver", "buyer", "seller"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TransactRequest {
    pub party_a: Party,
    pub party_b: Party,
    pub terms: serde_json::Value,
    pub context: Context,
}

// ---------------------------------------------------------------------------
// Unified ActRequest — dispatches to the right handler
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "act")]
pub enum ActRequest {
    #[serde(rename = "ATTEST")]
    Attest(AttestRequest),
    #[serde(rename = "EVALUATE")]
    Evaluate(EvaluateRequest),
    #[serde(rename = "TRANSACT")]
    Transact(TransactRequest),
}

impl ActRequest {
    pub fn act(&self) -> Act {
        match self {
            Self::Attest(_) => Act::Attest,
            Self::Evaluate(_) => Act::Evaluate,
            Self::Transact(_) => Act::Transact,
        }
    }

    pub fn context(&self) -> &Context {
        match self {
            Self::Attest(r) => &r.context,
            Self::Evaluate(r) => &r.context,
            Self::Transact(r) => &r.context,
        }
    }
}
