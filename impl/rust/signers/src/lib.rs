use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use ed25519_dalek::pkcs8::DecodePrivateKey;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Trait: sign the BLAKE3 canon hash with Ed25519
// ---------------------------------------------------------------------------

/// A signer takes 32 bytes (BLAKE3 hash of canonical NRF bytes) and returns
/// a 64-byte Ed25519 signature. The invariant:
///   NRF encode → BLAKE3 → Ed25519 sign
pub trait Signer {
    fn sign_canon_hash(&self, blake3_hash: &[u8; 32]) -> Result<Vec<u8>>;
}

/// Convenience: hash a `nrf_core::Value`, then sign.
pub fn sign_value(signer: &dyn Signer, value: &nrf_core::Value) -> Result<Vec<u8>> {
    let nrf_bytes = nrf_core::encode(value);
    let hash = blake3::hash(&nrf_bytes);
    signer.sign_canon_hash(hash.as_bytes())
}

/// Convenience: verify a signature over a `nrf_core::Value`.
pub fn verify_value(
    vk: &ed25519_dalek::VerifyingKey,
    value: &nrf_core::Value,
    sig_bytes: &[u8],
) -> bool {
    let nrf_bytes = nrf_core::encode(value);
    let hash = blake3::hash(&nrf_bytes);
    if let Ok(sig) = ed25519_dalek::Signature::from_slice(sig_bytes) {
        use ed25519_dalek::Verifier;
        return vk.verify(hash.as_bytes(), &sig).is_ok();
    }
    false
}

// ---------------------------------------------------------------------------
// Wire types for HTTP signers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRequest {
    /// hex-encoded BLAKE3 hash (32 bytes)
    pub hash_hex: String,
    pub kid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignResponse {
    /// base64 of raw Ed25519 signature (64 bytes)
    pub sig_b64: String,
}

// ---------------------------------------------------------------------------
// Local Ed25519 signer
// ---------------------------------------------------------------------------

pub struct LocalSigner {
    key: ed25519_dalek::SigningKey,
}

impl LocalSigner {
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self> {
        let key = ed25519_dalek::SigningKey::from_pkcs8_pem(pem)
            .map_err(|e| anyhow!("pkcs8 parse: {e}"))?;
        Ok(Self { key })
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            key: ed25519_dalek::SigningKey::from_bytes(bytes),
        }
    }

    pub fn generate() -> Self {
        let mut rng = rand_core::OsRng;
        let key = ed25519_dalek::SigningKey::generate(&mut rng);
        Self { key }
    }

    pub fn verifying_key(&self) -> ed25519_dalek::VerifyingKey {
        self.key.verifying_key()
    }

    pub fn public_key_b64(&self) -> String {
        general_purpose::STANDARD.encode(self.key.verifying_key().to_bytes())
    }
}

impl Signer for LocalSigner {
    fn sign_canon_hash(&self, blake3_hash: &[u8; 32]) -> Result<Vec<u8>> {
        use ed25519_dalek::Signer as _;
        Ok(self.key.sign(blake3_hash).to_bytes().to_vec())
    }
}

// ---------------------------------------------------------------------------
// HTTP signer (delegates to remote endpoint)
// ---------------------------------------------------------------------------

pub struct HttpSigner {
    pub endpoint: String,
    pub kid: Option<String>,
    pub auth_header: Option<String>,
}

impl Signer for HttpSigner {
    fn sign_canon_hash(&self, blake3_hash: &[u8; 32]) -> Result<Vec<u8>> {
        let client = reqwest::blocking::Client::new();
        let req = SignRequest {
            hash_hex: hex::encode(blake3_hash),
            kid: self.kid.clone(),
        };
        let mut rb = client.post(&self.endpoint).json(&req);
        if let Some(h) = &self.auth_header {
            rb = rb.header("Authorization", h);
        }
        let resp = rb.send()?;
        if !resp.status().is_success() {
            return Err(anyhow!("signer http status {}", resp.status()));
        }
        let sr: SignResponse = resp.json()?;
        let sig = general_purpose::STANDARD.decode(sr.sig_b64)?;
        if sig.len() != 64 {
            return Err(anyhow!("expected 64-byte ed25519 signature"));
        }
        Ok(sig)
    }
}

/// Cloudflare Workers convenience alias
pub type CloudflareSigner = HttpSigner;
