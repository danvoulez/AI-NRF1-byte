use nrf_core::{Value, rho};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Certified Runtime — BASE terrain
//
// The runtime attestation contract. This is the INTERFACE that lives in the
// BASE. Module implementations (runtime-llm, runtime-wasm, runtime-tee)
// plug into this trait.
//
// Two layers:
//   1. ρ-level (this crate): RuntimeInfo struct, canonical form, validation.
//      Every RuntimeInfo is ρ-normalized before entering a Receipt.
//      This guarantees: same runtime + same env → same bytes → same hash.
//
//   2. Module-level (implementations): How you actually GET the binary hash,
//      how you talk to the TEE, how you pin the LLM seed. That's domain logic.
//
// The rule: every Receipt MUST have a valid RuntimeInfo.
// A Receipt with a missing or non-canonical RuntimeInfo is unconstitutional.
// ---------------------------------------------------------------------------

// =========================================================================
// Errors
// =========================================================================

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("RT-001: binary_sha256 is empty — every runtime must identify its binary")]
    EmptyBinarySha,

    #[error("RT-002: runtime name is empty")]
    EmptyName,

    #[error("RT-003: runtime version is empty")]
    EmptyVersion,

    #[error("RT-004: ρ normalization failed on RuntimeInfo: {0}")]
    RhoFailed(String),

    #[error("RT-005: RuntimeInfo is not ρ-canonical: {0}")]
    NotCanonical(String),

    #[error("RT-006: attestation failed: {0}")]
    AttestationFailed(String),

    #[error("RT-007: cert at index {0} is empty")]
    EmptyCert(usize),
}

// =========================================================================
// RuntimeInfo — the canonical struct (ρ-validated)
//
// This is the same struct as receipt::RuntimeInfo but owned by the BASE
// runtime crate. The receipt crate should eventually re-export from here.
// For now they are structurally identical.
// =========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

impl RuntimeInfo {
    /// Validate that this RuntimeInfo is well-formed.
    /// This is the structural check — does it have the required fields?
    pub fn validate_structure(&self) -> Result<(), RuntimeError> {
        if self.name.is_empty() {
            return Err(RuntimeError::EmptyName);
        }
        if self.version.is_empty() {
            return Err(RuntimeError::EmptyVersion);
        }
        if self.binary_sha256.is_empty() {
            return Err(RuntimeError::EmptyBinarySha);
        }
        for (i, cert) in self.certs.iter().enumerate() {
            if cert.is_empty() {
                return Err(RuntimeError::EmptyCert(i));
            }
        }
        Ok(())
    }

    /// Convert to a ρ-canonical NRF Value.
    ///
    /// This is the byte-level representation that enters the Receipt hash.
    /// ρ guarantees: same RuntimeInfo → same bytes → same hash. Always.
    pub fn to_canonical_value(&self) -> Result<Value, RuntimeError> {
        let mut m = BTreeMap::new();

        m.insert("binary_sha256".into(), Value::String(self.binary_sha256.clone()));

        if !self.certs.is_empty() {
            let certs: Vec<Value> = self.certs.iter()
                .map(|c| Value::Bytes(c.clone()))
                .collect();
            m.insert("certs".into(), Value::Array(certs));
        }

        if !self.env.is_empty() {
            let env_map: BTreeMap<String, Value> = self.env.iter()
                .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                .collect();
            m.insert("env".into(), Value::Map(env_map));
        }

        if let Some(h) = &self.hal_ref {
            m.insert("hal_ref".into(), Value::String(h.clone()));
        }

        m.insert("name".into(), Value::String(self.name.clone()));
        m.insert("version".into(), Value::String(self.version.clone()));

        let v = Value::Map(m);

        // ρ-normalize: NFC strings, strip nulls, sort keys
        rho::normalize(&v)
            .map_err(|e| RuntimeError::RhoFailed(format!("{}", e)))
    }

    /// Compute the CID of this RuntimeInfo (ρ-canonical).
    ///
    /// This is the fingerprint of the execution environment.
    /// Two identical runtimes produce the same CID. Always.
    pub fn canonical_cid(&self) -> Result<String, RuntimeError> {
        let v = self.to_canonical_value()?;
        rho::canonical_cid(&v)
            .map_err(|e| RuntimeError::RhoFailed(format!("{}", e)))
    }

    /// Full validation: structure + ρ-canonicality.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        self.validate_structure()?;
        // Verify ρ-canonicality: normalize and check it doesn't change
        let v = self.to_canonical_value()?;
        rho::validate(&v)
            .map_err(|e| RuntimeError::NotCanonical(format!("{}", e)))?;
        Ok(())
    }
}

// =========================================================================
// RuntimeAttestation trait — the contract for MODULE implementations
//
// Any certified runtime must implement this trait.
// The BASE defines WHAT an attestation must produce.
// The MODULE defines HOW it produces it.
// =========================================================================

/// The input to a runtime attestation: what are we about to execute?
#[derive(Debug, Clone)]
pub struct AttestationRequest {
    pub input_cid: String,          // CID of the input being processed
    pub act: String,                // ATTEST | EVALUATE | TRANSACT
    pub policy_id: Option<String>,  // which policy is being applied
}

/// The output of a runtime attestation: proof of execution environment.
#[derive(Debug, Clone)]
pub struct AttestationResponse {
    pub info: RuntimeInfo,          // the canonical runtime info
    pub reasoning_cid: Option<String>, // CID of the reasoning output (if applicable)
}

/// The trait. Any certified runtime module implements this.
///
/// Implementations:
///   - `runtime-llm`: deterministic LLM (seed=0, temp=0, top_p=1)
///   - `runtime-wasm`: sandboxed WASM with memory limits
///   - `runtime-tee`: hardware TEE attestation (SGX, TrustZone, etc.)
///
/// The contract:
///   1. `attest()` MUST return a valid RuntimeInfo (passes validate())
///   2. Same input → same RuntimeInfo → same CID (determinism)
///   3. `binary_sha256` MUST be the actual SHA-256 of the running binary
///   4. `certs` SHOULD include any hardware attestation certificates
pub trait RuntimeAttestation: Send + Sync {
    /// Produce an attestation for the current execution environment.
    fn attest(&self, req: &AttestationRequest) -> Result<AttestationResponse, RuntimeError>;

    /// Human-readable name of this runtime implementation.
    fn runtime_name(&self) -> &str;

    /// Version of this runtime implementation.
    fn runtime_version(&self) -> &str;
}

// =========================================================================
// Built-in: SelfAttestation — the simplest runtime (for development/testing)
//
// Reports its own binary hash and environment. No hardware attestation.
// This is the "I trust myself" runtime. Fine for development.
// In production, use a real TEE or deterministic runtime module.
// =========================================================================

pub struct SelfAttestation {
    pub binary_sha256: String,
    pub name: String,
    pub version: String,
    pub env: BTreeMap<String, String>,
}

impl SelfAttestation {
    pub fn new(binary_sha256: &str) -> Self {
        Self {
            binary_sha256: binary_sha256.to_string(),
            name: "self-attestation".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            env: BTreeMap::new(),
        }
    }
}

impl RuntimeAttestation for SelfAttestation {
    fn attest(&self, _req: &AttestationRequest) -> Result<AttestationResponse, RuntimeError> {
        let info = RuntimeInfo {
            name: self.name.clone(),
            version: self.version.clone(),
            binary_sha256: self.binary_sha256.clone(),
            hal_ref: None,
            env: self.env.clone(),
            certs: vec![],
        };
        info.validate()?;
        Ok(AttestationResponse {
            info,
            reasoning_cid: None,
        })
    }

    fn runtime_name(&self) -> &str {
        &self.name
    }

    fn runtime_version(&self) -> &str {
        &self.version
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_info() -> RuntimeInfo {
        RuntimeInfo {
            name: "test-runtime".into(),
            version: "0.1.0".into(),
            binary_sha256: "abcdef1234567890".into(),
            hal_ref: None,
            env: BTreeMap::new(),
            certs: vec![],
        }
    }

    #[test]
    fn runtime_info_validates_ok() {
        let info = test_info();
        assert!(info.validate().is_ok(), "valid RuntimeInfo must pass validation");
    }

    #[test]
    fn runtime_info_rejects_empty_name() {
        let mut info = test_info();
        info.name = String::new();
        let err = info.validate_structure();
        assert!(err.is_err());
        assert!(format!("{}", err.unwrap_err()).contains("RT-002"));
    }

    #[test]
    fn runtime_info_rejects_empty_version() {
        let mut info = test_info();
        info.version = String::new();
        let err = info.validate_structure();
        assert!(err.is_err());
        assert!(format!("{}", err.unwrap_err()).contains("RT-003"));
    }

    #[test]
    fn runtime_info_rejects_empty_binary_sha() {
        let mut info = test_info();
        info.binary_sha256 = String::new();
        let err = info.validate_structure();
        assert!(err.is_err());
        assert!(format!("{}", err.unwrap_err()).contains("RT-001"));
    }

    #[test]
    fn runtime_info_rejects_empty_cert() {
        let mut info = test_info();
        info.certs = vec![vec![0xDE, 0xAD], vec![]]; // second cert is empty
        let err = info.validate_structure();
        assert!(err.is_err());
        assert!(format!("{}", err.unwrap_err()).contains("RT-007"));
    }

    #[test]
    fn runtime_info_canonical_cid_deterministic() {
        let info = test_info();
        let cid1 = info.canonical_cid().unwrap();
        let cid2 = info.canonical_cid().unwrap();
        assert_eq!(cid1, cid2, "same RuntimeInfo must produce same CID");
        assert!(cid1.starts_with("b3:"), "CID must start with b3:");
    }

    #[test]
    fn runtime_info_canonical_value_is_rho_stable() {
        let info = test_info();
        let v = info.to_canonical_value().unwrap();
        // ρ(ρ(v)) = ρ(v) — idempotent
        let v2 = rho::normalize(&v).unwrap();
        assert_eq!(v, v2, "RuntimeInfo canonical value must be ρ-stable");
    }

    #[test]
    fn runtime_info_env_changes_cid() {
        let info1 = test_info();
        let mut info2 = test_info();
        info2.env.insert("CUDA_VERSION".into(), "12.0".into());
        let cid1 = info1.canonical_cid().unwrap();
        let cid2 = info2.canonical_cid().unwrap();
        assert_ne!(cid1, cid2, "different env must produce different CID");
    }

    #[test]
    fn self_attestation_works() {
        let rt = SelfAttestation::new("deadbeef1234");
        let req = AttestationRequest {
            input_cid: "b3:0000".into(),
            act: "EVALUATE".into(),
            policy_id: None,
        };
        let resp = rt.attest(&req).unwrap();
        assert_eq!(resp.info.binary_sha256, "deadbeef1234");
        assert!(resp.info.validate().is_ok());
    }

    #[test]
    fn self_attestation_deterministic() {
        let rt = SelfAttestation::new("deadbeef1234");
        let req = AttestationRequest {
            input_cid: "b3:0000".into(),
            act: "ATTEST".into(),
            policy_id: None,
        };
        let r1 = rt.attest(&req).unwrap();
        let r2 = rt.attest(&req).unwrap();
        let cid1 = r1.info.canonical_cid().unwrap();
        let cid2 = r2.info.canonical_cid().unwrap();
        assert_eq!(cid1, cid2, "same runtime + same input → same CID");
    }
}
