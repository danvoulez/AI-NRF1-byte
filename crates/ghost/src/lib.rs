use serde::{Serialize, Deserialize};
use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use nrf1::Value;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Ghost — Write-Before-Execute state machine (BASE terrain)
//
// Every execution MUST start as a Ghost(pending).
// The ghost is immutable evidence of intent.
//
// State machine:
//   WBE ──create──> GHOST[pending]
//   GHOST[pending] ──promote──> links to final RECEIPT (ghost_cid carried)
//   GHOST[pending] ──expire──>  GHOST[expired] (cause recorded)
//
// Same fractal: Value → NRF → BLAKE3 → CID → Ed25519 → URL
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GhostStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "expired")]
    Expired,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ExpireCause {
    #[serde(rename = "timeout")]
    Timeout,
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "drift")]
    Drift,
    #[serde(rename = "none")]
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Wbe {
    pub who: String,        // DID of the actor
    pub what: String,       // description of the intended action
    pub when: i64,          // unix nanos of intent
    pub intent: String,     // structured intent (act type or free text)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Ghost {
    pub v: String,                              // "ghost.v1"
    pub ghost_cid: String,                      // b3:<hex> over NRF(without sig)
    pub t: i64,                                 // unix nanos
    pub status: GhostStatus,                    // pending | expired
    pub wbe: Wbe,                               // write-before-execute record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<ExpireCause>,             // only set when expired
    pub nonce: Vec<u8>,                         // 16 bytes
    pub url: String,                            // rich URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<Vec<u8>>,                   // Ed25519(BLAKE3(NRF(without sig)))
}

// ---------------------------------------------------------------------------
// Ghost reference — carried by the Receipt that promotes this ghost
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GhostRef {
    pub ghost_id: String,   // storage ID
    pub ghost_cid: String,  // b3:<hex> — verifiable link
}

// ---------------------------------------------------------------------------
// impl Ghost
// ---------------------------------------------------------------------------

impl Ghost {
    /// Create a new pending ghost (WBE step).
    pub fn new_pending(wbe: Wbe, nonce: Vec<u8>, url: String) -> Self {
        let mut g = Ghost {
            v: "ghost.v1".into(),
            ghost_cid: String::new(),
            t: wbe.when,
            status: GhostStatus::Pending,
            wbe,
            cause: None,
            nonce,
            url,
            sig: None,
        };
        g.ghost_cid = g.compute_cid();
        g
    }

    /// Expire this ghost with a cause.
    pub fn expire(&mut self, cause: ExpireCause) {
        self.status = GhostStatus::Expired;
        self.cause = Some(cause);
        // CID changes because status changed — recompute
        self.ghost_cid = self.compute_cid();
        self.sig = None; // must re-sign after mutation
    }

    /// Build a GhostRef for embedding in a Receipt that promotes this ghost.
    pub fn as_ref(&self, storage_id: &str) -> GhostRef {
        GhostRef {
            ghost_id: storage_id.into(),
            ghost_cid: self.ghost_cid.clone(),
        }
    }

    /// Canonical NRF map without sig and ghost_cid (the hash preimage).
    pub fn nrf_without_sig(&self) -> Value {
        use Value::*;
        let mut m = BTreeMap::new();

        if let Some(c) = &self.cause {
            let s = match c {
                ExpireCause::Timeout => "timeout",
                ExpireCause::Canceled => "canceled",
                ExpireCause::Drift => "drift",
                ExpireCause::None => "none",
            };
            m.insert("cause".into(), String(s.into()));
        }

        m.insert("nonce".into(), Bytes(self.nonce.clone()));

        let status_str = match self.status {
            GhostStatus::Pending => "pending",
            GhostStatus::Expired => "expired",
        };
        m.insert("status".into(), String(status_str.into()));

        m.insert("t".into(), Int(self.t));
        m.insert("url".into(), String(self.url.clone()));
        m.insert("v".into(), String(self.v.clone()));

        // wbe as sub-map
        let mut wm = BTreeMap::new();
        wm.insert("intent".into(), String(self.wbe.intent.clone()));
        wm.insert("what".into(), String(self.wbe.what.clone()));
        wm.insert("when".into(), Int(self.wbe.when));
        wm.insert("who".into(), String(self.wbe.who.clone()));
        m.insert("wbe".into(), Map(wm));

        // ghost_cid and sig deliberately omitted
        Map(m)
    }

    pub fn compute_cid(&self) -> String {
        nrf1::blake3_cid(&self.nrf_without_sig())
    }

    pub fn sign(&mut self, sk: &SigningKey) {
        let bytes = nrf1::encode_stream(&self.nrf_without_sig());
        let digest = blake3::hash(&bytes);
        let sig = sk.sign(digest.as_bytes());
        self.sig = Some(sig.to_bytes().to_vec());
    }

    pub fn verify(&self, vk: &VerifyingKey) -> bool {
        let bytes = nrf1::encode_stream(&self.nrf_without_sig());
        let digest = blake3::hash(&bytes);
        if let Some(sig) = &self.sig {
            if let Ok(s) = Signature::from_slice(sig) {
                return vk.verify(digest.as_bytes(), &s).is_ok();
            }
        }
        false
    }

    pub fn verify_integrity(&self) -> Result<(), &'static str> {
        if self.compute_cid() != self.ghost_cid {
            return Err("ghost_cid mismatch");
        }
        if self.status == GhostStatus::Pending && self.cause.is_some() {
            return Err("pending ghost must not have a cause");
        }
        if self.status == GhostStatus::Expired && self.cause.is_none() {
            return Err("expired ghost must have a cause");
        }
        Ok(())
    }
}
