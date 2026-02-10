use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use nrf1::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuntimeInfo {
    pub name: String,
    pub version: String,
    pub binary_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hal_ref: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certs: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostInfo {
    pub budget: u64,
    pub counter: u64,
    pub cost_ms: u64,
    pub window_day: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainInfo {
    pub prev_cid: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skips: Vec<Option<String>>,
    pub link_hash: String,
}

// ---------------------------------------------------------------------------
// Receipt — THE canonical artifact (BASE terrain)
//
// Every module emits one. Every product is built from these.
// The fractal invariant: Value → NRF → BLAKE3 → CID → Ed25519 → URL
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Receipt {
    // --- identity ---
    pub v: String,           // "receipt-v1"
    pub receipt_cid: String, // b3:<hex> over NRF(without sig)

    // --- time ---
    pub t: i64, // unix nanos

    // --- parties ---
    pub issuer_did: String, // did:method:... (who signed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_did: Option<String>, // did of the subject party
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kid: Option<String>, // key-id for rotation

    // --- act (the 3 basis vectors) ---
    pub act: String,     // ATTEST | EVALUATE | TRANSACT
    pub subject: String, // CID of what's being acted on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>, // ALLOW | DENY | REQUIRE | GHOST
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effects: Option<Value>, // Map of effects; MUST be null/None for GHOST

    // --- payload ---
    pub body: Value,      // canonical NRF value
    pub body_cid: String, // b3:<hex> of encode(body)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs_cid: Option<String>, // b3:<hex> of input context

    // --- policy ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>, // e.g. "pack-compliance/eu-ai-act@1"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_cid: Option<String>, // b3:<hex> of ReasoningBit

    // --- permit (accountability) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permit_cid: Option<String>, // b3:<hex> of the Permit that authorized this

    // --- pipeline (free composition) ---
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pipeline_prev: Vec<String>, // CIDs of prior act receipts in this pipeline

    // --- runtime ---
    pub rt: RuntimeInfo,

    // --- chain ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>, // previous receipt CID (linear chain)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<ChainInfo>, // skip-list enrichment

    // --- ghost (WBE enrichment) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ghost: Option<GhostInfo>,

    // --- entropy ---
    pub nonce: Vec<u8>, // 16 bytes

    // --- location ---
    pub url: String, // rich URL: base#cid=...&did=...&act=...

    // --- signature (omitted from NRF hash) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<Vec<u8>>, // Ed25519(BLAKE3(NRF(without sig)))
}

// ---------------------------------------------------------------------------
// Rich URL builder
// ---------------------------------------------------------------------------

pub fn rich_url(base: &str, cid: &str, did: &str, act: &str) -> String {
    format!(
        "{}#cid={}&did={}&act={}",
        base.trim_end_matches('/'),
        cid,
        did,
        act
    )
}

// ---------------------------------------------------------------------------
// Link hash (chain integrity)
// ---------------------------------------------------------------------------

pub fn link_hash(
    cid: &str,
    body_cid: &str,
    prev: Option<&str>,
    skips: &[Option<String>],
) -> String {
    use blake3::Hasher;
    let mut h = Hasher::new();
    h.update(cid.as_bytes());
    h.update(body_cid.as_bytes());
    if let Some(p) = prev {
        h.update(p.as_bytes());
    } else {
        h.update(b"");
    }
    for s in skips {
        match s {
            Some(x) => {
                h.update(x.as_bytes());
            }
            None => {
                h.update(b"\x00");
            }
        }
    }
    format!("b3:{}", h.finalize().to_hex())
}

// ---------------------------------------------------------------------------
// impl Receipt
// ---------------------------------------------------------------------------

impl Receipt {
    /// All fields EXCEPT `sig` and `receipt_cid`, serialized as a canonical
    /// NRF Map with deterministic sorted keys. This is the hash preimage.
    ///
    /// The fractal rule: every CID in this map is itself b3(NRF(value)).
    pub fn nrf_without_sig(&self) -> Value {
        use Value::*;
        let mut m = BTreeMap::new();

        // identity
        m.insert("v".into(), String(self.v.clone()));

        // time
        m.insert("t".into(), Int(self.t));

        // parties
        m.insert("issuer_did".into(), String(self.issuer_did.clone()));
        if let Some(sd) = &self.subject_did {
            m.insert("subject_did".into(), String(sd.clone()));
        }
        if let Some(k) = &self.kid {
            m.insert("kid".into(), String(k.clone()));
        }

        // act
        m.insert("act".into(), String(self.act.clone()));
        m.insert("subject".into(), String(self.subject.clone()));
        if let Some(d) = &self.decision {
            m.insert("decision".into(), String(d.clone()));
        }
        if let Some(e) = &self.effects {
            m.insert("effects".into(), e.clone());
        }

        // payload
        m.insert("body".into(), self.body.clone());
        m.insert("body_cid".into(), String(self.body_cid.clone()));
        if let Some(ic) = &self.inputs_cid {
            m.insert("inputs_cid".into(), String(ic.clone()));
        }

        // policy
        if let Some(p) = &self.policy {
            m.insert("policy".into(), String(p.clone()));
        }
        if let Some(rc) = &self.reasoning_cid {
            m.insert("reasoning_cid".into(), String(rc.clone()));
        }

        // permit
        if let Some(pc) = &self.permit_cid {
            m.insert("permit_cid".into(), String(pc.clone()));
        }

        // pipeline
        if !self.pipeline_prev.is_empty() {
            let arr: Vec<Value> = self
                .pipeline_prev
                .iter()
                .map(|c| String(c.clone()))
                .collect();
            m.insert("pipeline_prev".into(), Array(arr));
        }

        // runtime
        let mut rt = BTreeMap::new();
        rt.insert(
            "binary_sha256".into(),
            String(self.rt.binary_sha256.clone()),
        );
        if !self.rt.certs.is_empty() {
            let certs: Vec<Value> = self.rt.certs.iter().map(|c| Bytes(c.clone())).collect();
            rt.insert("certs".into(), Array(certs));
        }
        if !self.rt.env.is_empty() {
            let env_map: BTreeMap<std::string::String, Value> = self
                .rt
                .env
                .iter()
                .map(|(k, v)| (k.clone(), String(v.clone())))
                .collect();
            rt.insert("env".into(), Map(env_map));
        }
        if let Some(h) = &self.rt.hal_ref {
            rt.insert("hal_ref".into(), String(h.clone()));
        }
        rt.insert("name".into(), String(self.rt.name.clone()));
        rt.insert("version".into(), String(self.rt.version.clone()));
        m.insert("rt".into(), Map(rt));

        // chain
        if let Some(p) = &self.prev {
            m.insert("prev".into(), String(p.clone()));
        }

        // ghost (as sub-map)
        if let Some(g) = &self.ghost {
            let mut gm = BTreeMap::new();
            gm.insert("budget".into(), Int(g.budget as i64));
            gm.insert("cost_ms".into(), Int(g.cost_ms as i64));
            gm.insert("counter".into(), Int(g.counter as i64));
            gm.insert("window_day".into(), Int(g.window_day as i64));
            m.insert("ghost".into(), Map(gm));
        }

        // entropy
        m.insert("nonce".into(), Bytes(self.nonce.clone()));

        // location
        m.insert("url".into(), String(self.url.clone()));

        // receipt_cid and sig deliberately omitted — they are computed over this map
        Map(m)
    }

    pub fn compute_cid(&self) -> String {
        nrf1::blake3_cid(&self.nrf_without_sig())
    }

    pub fn compute_body_cid(&self) -> String {
        nrf1::blake3_cid(&self.body)
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

    /// Verify internal consistency: body_cid matches body, receipt_cid matches NRF.
    pub fn verify_integrity(&self) -> Result<(), &'static str> {
        if self.compute_body_cid() != self.body_cid {
            return Err("body_cid mismatch");
        }
        if self.compute_cid() != self.receipt_cid {
            return Err("receipt_cid mismatch");
        }
        // GHO-001: if GHOST then effects must be None
        if self.decision.as_deref() == Some("GHOST") && self.effects.is_some() {
            return Err("GHOST receipt must have null effects");
        }
        Ok(())
    }
}
