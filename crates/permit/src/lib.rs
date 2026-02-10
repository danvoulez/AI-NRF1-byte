use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use nrf1::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Permit — the accountability closer (BASE terrain)
//
// A Permit is a signed, time-bounded, hash-pinned authorization.
// It says: "TDLN decided ALLOW for this specific input, and the executor
// MUST verify this permit before running."
//
// Without a permit, "ALLOW" is a claim. With a permit, "ALLOW" is math.
//
// Same fractal: Value → NRF → BLAKE3 → CID → Ed25519 → URL
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Permit {
    pub v: String,          // "permit-v1"
    pub permit_cid: String, // b3:<hex> over NRF(without sig)

    // --- what was decided ---
    pub request_cid: String, // CID of the original request
    pub decision: String,    // ALLOW (only ALLOW gets a permit)
    pub input_hash: String,  // b3:<hex> of the input that was evaluated

    // --- who authorized ---
    pub issuer_did: String, // DID of the authority (TDLN)

    // --- bounds ---
    pub issued_at: i64,  // unix nanos
    pub expires_at: i64, // unix nanos — permit is invalid after this

    // --- scope ---
    pub act: String, // ATTEST | EVALUATE | TRANSACT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>, // which policy was applied

    // --- signature (omitted from NRF hash) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<Vec<u8>>, // Ed25519(BLAKE3(NRF(without sig)))
}

impl Permit {
    /// Canonical NRF map without sig and permit_cid (the hash preimage).
    pub fn nrf_without_sig(&self) -> Value {
        use Value::*;
        let mut m = BTreeMap::new();
        m.insert("act".into(), String(self.act.clone()));
        m.insert("decision".into(), String(self.decision.clone()));
        m.insert("expires_at".into(), Int(self.expires_at));
        m.insert("input_hash".into(), String(self.input_hash.clone()));
        m.insert("issued_at".into(), Int(self.issued_at));
        m.insert("issuer_did".into(), String(self.issuer_did.clone()));
        if let Some(p) = &self.policy {
            m.insert("policy".into(), String(p.clone()));
        }
        m.insert("request_cid".into(), String(self.request_cid.clone()));
        m.insert("v".into(), String(self.v.clone()));
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

    pub fn is_expired(&self, now_nanos: i64) -> bool {
        now_nanos > self.expires_at
    }
}

// ---------------------------------------------------------------------------
// Verification — the executor calls this before running
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum PermitError {
    NotAllowed,
    Expired,
    InputMismatch,
    CidMismatch,
    BadSignature,
    MissingSig,
}

impl std::fmt::Display for PermitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAllowed => write!(f, "permit decision is not ALLOW"),
            Self::Expired => write!(f, "permit has expired"),
            Self::InputMismatch => write!(f, "input hash does not match permit"),
            Self::CidMismatch => write!(f, "permit_cid does not match computed CID"),
            Self::BadSignature => write!(f, "Ed25519 signature verification failed"),
            Self::MissingSig => write!(f, "permit has no signature"),
        }
    }
}

impl std::error::Error for PermitError {}

/// Verify a permit against the actual input and the authority's public key.
///
/// This is the function the executor calls. If it returns Ok(()), proceed.
/// If it returns Err, do NOT execute.
pub fn verify_permit(
    permit: &Permit,
    input_hash: &str,
    now_nanos: i64,
    authority_key: &VerifyingKey,
) -> Result<(), PermitError> {
    // 1. Decision must be ALLOW
    if permit.decision != "ALLOW" {
        return Err(PermitError::NotAllowed);
    }

    // 2. Not expired
    if permit.is_expired(now_nanos) {
        return Err(PermitError::Expired);
    }

    // 3. Input hash matches
    if permit.input_hash != input_hash {
        return Err(PermitError::InputMismatch);
    }

    // 4. CID integrity
    if permit.compute_cid() != permit.permit_cid {
        return Err(PermitError::CidMismatch);
    }

    // 5. Signature verification (Ed25519 over BLAKE3 of canonical NRF)
    let sig_bytes = permit.sig.as_ref().ok_or(PermitError::MissingSig)?;
    let bytes = nrf1::encode_stream(&permit.nrf_without_sig());
    let digest = blake3::hash(&bytes);
    let sig = Signature::from_slice(sig_bytes).map_err(|_| PermitError::BadSignature)?;
    authority_key
        .verify(digest.as_bytes(), &sig)
        .map_err(|_| PermitError::BadSignature)?;

    Ok(())
}
