use serde::{Serialize, Deserialize};
use blake3::Hasher;

// ---------------------------------------------------------------------------
// UBL Capsule v1 — BASE terrain
//
// The canonical wire format. A Capsule is a self-contained, signed,
// verifiable envelope carrying an NRF-encoded payload through the pipeline.
//
// Absorbed from UBL-Capsule_v1_SPEC.md into the BASE crate.
// Vocabulary aligned to Constitution of the Base:
//   - Decisions: ALLOW / DENY / REQUIRE / GHOST (not ACK/NACK/ASK)
//   - Acts:      ATTEST / EVALUATE / TRANSACT (not EVAL/BUNDLE/TRACE/QUERY)
//   - ρ:         All values MUST be ρ-normalized before capsule ID computation
//
// Same fractal: ρ(value) → encode → BLAKE3 → CID → Ed25519 sig → URL
// ---------------------------------------------------------------------------

pub const CAPSULE_VERSION: &str = "ubl-capsule/1.0";
pub const CAPSULE_DOMAIN: &str = "ubl-capsule/1.0";
pub const HOP_DOMAIN: &str = "ubl-receipt/1.0";

// ---------------------------------------------------------------------------
// Proof levels (config flag — determines how much transport proof is produced)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum ProofLevel {
    Receipt,    // cheapest: engine receipt with hash chain + signature
    Sirp,       // medium: INTENT + RESULT capsules, delivery + execution receipts
    Bundle,     // full: everything + offline ZIP with manifest, policy, EER, sigs, QR
}

// ---------------------------------------------------------------------------
// Capsule roles (SIRP positions)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum CapsuleRole {
    Intent,     // "I want to do X" — sent before execution
    Result,     // "X happened" — sent after execution
    Delivery,   // receiver acknowledges receipt of a capsule
    Execution,  // executor confirms completion
}

// ---------------------------------------------------------------------------
// Envelope type (what the capsule carries — from capsule spec env.t)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum EnvelopeType {
    Record,     // single act record
    Bundle,     // offline-verifiable bundle
    Trace,      // distributed trace / audit trail
    Query,      // read-only query
}

// ---------------------------------------------------------------------------
// Signature algorithm (Ed25519 now, Dilithium3 post-quantum future)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum SigAlg {
    Ed25519,
    Dilithium3,
}

// ---------------------------------------------------------------------------
// Header — stable routing fields (from capsule spec hdr)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Header {
    pub src: String,                    // sender DID (ASCII-only)
    pub dst: String,                    // receiver DID (ASCII-only)
    pub nonce: Vec<u8>,                 // 16 bytes random
    pub exp: i64,                       // expiry as epoch-nanos
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chan: Option<String>,           // optional channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ts: Option<i64>,               // optional sender timestamp (epoch-nanos)
}

// ---------------------------------------------------------------------------
// Envelope — semantic payload (from capsule spec env, using our vocabulary)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Intent {
    pub kind: String,                   // ATTEST | EVALUATE | TRANSACT
    pub name: String,                   // human-readable act name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Decision {
    pub verdict: String,                // ALLOW | DENY | REQUIRE | GHOST
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Evidence {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cids: Vec<Vec<u8>>,            // BLAKE3 hashes (32 bytes each)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meta {
    pub app: String,
    pub tenant: String,
    pub user: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Links {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<Vec<u8>>,          // previous capsule ID (32 bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<Vec<u8>>,         // trace root ID (32 bytes)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Envelope {
    pub t: EnvelopeType,                // record | bundle | trace | query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<serde_json::Value>,
    pub intent: Intent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<serde_json::Value>,
    pub decision: Decision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<Evidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<Links>,
}

// ---------------------------------------------------------------------------
// Seal — cryptographic binding (from capsule spec seal)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Seal {
    pub alg: SigAlg,                    // Ed25519 (now) | Dilithium3 (future)
    pub kid: String,                    // DID#key-id (ASCII-only)
    pub domain: String,                 // "ubl-capsule/1.0" — domain separation
    pub scope: String,                  // "capsule"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,            // optional: bind to dst
    pub sig: Vec<u8>,                   // Ed25519 = 64 bytes
}

// ---------------------------------------------------------------------------
// Hop Receipt — transport-layer receipt for each relay node
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HopReceipt {
    pub of: Vec<u8>,                    // capsule ID this hop receipts (32 bytes)
    pub prev: Vec<u8>,                  // previous hop receipt hash (32 bytes, or zeros for first)
    pub kind: String,                   // "relay" | "deliver" | "execute"
    pub node: String,                   // node DID (ASCII-only)
    pub ts: i64,                        // epoch-nanos
    pub sig: Vec<u8>,                   // Ed25519 over BLAKE3({domain, of, prev, kind, node, ts})
}

impl HopReceipt {
    /// Compute the signature preimage for a hop receipt.
    pub fn sig_preimage(&self) -> Vec<u8> {
        let mut h = Hasher::new();
        h.update(HOP_DOMAIN.as_bytes());
        h.update(&self.of);
        h.update(&self.prev);
        h.update(self.kind.as_bytes());
        h.update(self.node.as_bytes());
        h.update(&self.ts.to_be_bytes());
        h.finalize().as_bytes().to_vec()
    }

    /// Verify this hop receipt's signature.
    pub fn verify(&self, vk: &ed25519_dalek::VerifyingKey) -> bool {
        use ed25519_dalek::Verifier;
        let digest = self.sig_preimage();
        if let Ok(sig) = ed25519_dalek::Signature::from_slice(&self.sig) {
            return vk.verify(&digest, &sig).is_ok();
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Capsule — the full wire artifact
//
// id = BLAKE3(NRF(capsule \ {id, seal.sig, receipts[*].sig}))
// seal.sig signs BLAKE3(NRF({domain, id, hdr, env}))
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Capsule {
    pub v: String,                      // "ubl-capsule/1.0"
    pub id: Vec<u8>,                    // BLAKE3 hash (32 bytes)
    pub hdr: Header,
    pub env: Envelope,
    pub seal: Seal,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<HopReceipt>,
}

impl Capsule {
    /// Capsule ID as b3:<hex> string.
    pub fn id_cid(&self) -> String {
        format!("b3:{}", hex::encode(&self.id))
    }

    /// Compute the seal signature preimage:
    /// BLAKE3(domain || id || hdr_json || env_json)
    ///
    /// Domain separation ensures a signature over a capsule cannot be
    /// replayed as a signature over a different artifact type.
    pub fn seal_preimage(&self) -> Vec<u8> {
        let mut h = Hasher::new();
        h.update(CAPSULE_DOMAIN.as_bytes());
        h.update(&self.id);
        let hdr_bytes = serde_json::to_vec(&self.hdr).unwrap_or_default();
        h.update(&hdr_bytes);
        let env_bytes = serde_json::to_vec(&self.env).unwrap_or_default();
        h.update(&env_bytes);
        h.finalize().as_bytes().to_vec()
    }

    /// Verify the seal signature against the sender's public key.
    pub fn verify_seal(&self, vk: &ed25519_dalek::VerifyingKey) -> bool {
        use ed25519_dalek::Verifier;
        let digest = self.seal_preimage();
        if let Ok(sig) = ed25519_dalek::Signature::from_slice(&self.seal.sig) {
            return vk.verify(&digest, &sig).is_ok();
        }
        false
    }

    /// Verify the entire hop receipt chain.
    /// Each hop's `prev` must match the hash of the previous hop's sig_preimage,
    /// and the first hop's `of` must match the capsule ID.
    pub fn verify_hop_chain(&self) -> bool {
        for hop in &self.receipts {
            if hop.of != self.id {
                return false;
            }
        }
        // Chain: each hop[i].prev == BLAKE3(hop[i-1].sig_preimage()) for i > 0
        for i in 1..self.receipts.len() {
            let prev_digest = blake3::hash(&self.receipts[i - 1].sig_preimage());
            if self.receipts[i].prev != prev_digest.as_bytes().to_vec() {
                return false;
            }
        }
        true
    }

    /// Check structural invariants from the capsule spec:
    /// - GHOST verdict requires links.prev (ghost pending reference)
    /// - ALLOW/DENY verdict requires evidence to be present (can be empty)
    pub fn check_invariants(&self) -> Result<(), &'static str> {
        if self.v != CAPSULE_VERSION {
            return Err("capsule version mismatch");
        }
        if self.seal.domain != CAPSULE_DOMAIN {
            return Err("seal domain mismatch");
        }
        match self.env.decision.verdict.as_str() {
            "GHOST" => {
                // GHOST requires links.prev (the pending ghost reference)
                match &self.env.links {
                    Some(links) if links.prev.is_some() => {}
                    _ => return Err("GHOST verdict requires links.prev"),
                }
            }
            "ALLOW" | "DENY" => {
                // ALLOW/DENY require evidence to be present (can be empty)
                if self.env.evidence.is_none() {
                    return Err("ALLOW/DENY verdict requires evidence field");
                }
            }
            "REQUIRE" => {} // REQUIRE has no structural constraint beyond the decision
            other => return Err(if other.is_empty() {
                "empty verdict"
            } else {
                "unknown verdict"
            }),
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SIRP flow: INTENT → DELIVERY → EXECUTION → RESULT
// A complete SIRP exchange is 4 capsules. Each links to the previous via CID.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SirpFlow {
    pub intent: Capsule,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Capsule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<Capsule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Capsule>,
}

impl SirpFlow {
    pub fn new(intent: Capsule) -> Self {
        Self { intent, delivery: None, execution: None, result: None }
    }

    pub fn is_complete(&self) -> bool {
        self.delivery.is_some() && self.execution.is_some() && self.result.is_some()
    }
}
