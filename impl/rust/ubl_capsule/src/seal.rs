//! Seal — author signature over `{domain, id, hdr, env}`.
//!
//! Domain separation: the signed payload is
//!   `blake3(nrf.encode({domain, id, hdr, env}))`
//! with checks for `domain == "ubl-capsule/1.0"`, `scope == "capsule"`,
//! and `aud == hdr.dst` (if aud is present).

use crate::id::compute_id;
use crate::types::{Capsule, DOMAIN};
use nrf_core::Value;
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[cfg(feature = "metrics")]
use crate::metrics_support::ensure_exporter;
#[cfg(feature = "metrics")]
use metrics::{counter, histogram};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SealError {
    #[error("Err.Seal.BadDomain: expected '{}'", DOMAIN)]
    BadDomain,
    #[error("Err.Seal.BadScope: expected 'capsule'")]
    BadScope,
    #[error("Err.Seal.BadAudience: seal.aud does not match hdr.dst")]
    BadAudience,
    #[error("Err.Seal.BadSignature")]
    BadSignature,
    #[error("Err.Seal.IdMismatch: capsule.id does not match computed ID")]
    IdMismatch,
    #[error("Err.Hdr.Expired: capsule expired at {exp}, now={now}")]
    Expired { exp: i64, now: i64 },
}

#[cfg(feature = "metrics")]
impl SealError {
    fn code(&self) -> &'static str {
        match self {
            SealError::BadDomain => "BadDomain",
            SealError::BadScope => "BadScope",
            SealError::BadAudience => "BadAudience",
            SealError::BadSignature => "BadSignature",
            SealError::IdMismatch => "IdMismatch",
            SealError::Expired { .. } => "Expired",
        }
    }
}

/// Options for capsule verification.
#[derive(Default)]
pub struct VerifyOpts {
    /// Allowed clock skew in nanoseconds (default: 0).
    pub allowed_skew_ns: i64,
    /// Override current time (epoch-nanos). When `None`, uses host time.
    pub now_ns: Option<i64>,
}

/// Current time as epoch-nanoseconds (i64).
pub fn now_nanos_i64() -> i64 {
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = (d.as_secs() as u128)
        .saturating_mul(1_000_000_000)
        .saturating_add(d.subsec_nanos() as u128);
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

// ---------------------------------------------------------------------------
// Sign
// ---------------------------------------------------------------------------

/// Sign a capsule: compute its ID, build the signing payload, and produce
/// an Ed25519 signature. Mutates `c.id` and `c.seal.sig` in place.
///
/// Returns Err if env.body contains floats or non-i64 numbers.
/// Canon 2: no floats. Canon 3: ρ is the law. Canon 6: reject, never degrade.
#[cfg_attr(
    feature = "obs",
    tracing::instrument(level = "debug", skip_all, fields(src = %c.hdr.src, act = %c.hdr.act))
)]
pub fn sign(c: &mut Capsule, sk: &ed25519_dalek::SigningKey) -> Result<(), String> {
    // 1. Compute stable ID (Canon 3: ρ → encode → BLAKE3)
    c.id = compute_id(c)?;

    // 2. Build signing payload: {domain, id, hdr, env}
    let payload_hash = signing_hash(c)?;

    // 3. Sign
    use ed25519_dalek::Signer;
    let sig = sk.sign(&payload_hash);
    c.seal.sig = sig.to_bytes();
    Ok(())
}

/// Verify a capsule's seal:
///   1. domain == "ubl-capsule/1.0"
///   2. scope == "capsule"
///   3. aud == hdr.dst (if aud present)
///   4. id matches computed ID
///   5. Ed25519 signature valid
pub fn verify(c: &Capsule, pk: &ed25519_dalek::VerifyingKey) -> Result<(), SealError> {
    verify_with_opts(c, pk, &VerifyOpts::default())
}

/// Verify with configurable options (clock skew, etc.).
#[cfg_attr(
    feature = "obs",
    tracing::instrument(level = "debug", skip_all, fields(src = %c.hdr.src, act = %c.hdr.act))
)]
pub fn verify_with_opts(
    c: &Capsule,
    pk: &ed25519_dalek::VerifyingKey,
    opts: &VerifyOpts,
) -> Result<(), SealError> {
    #[cfg(feature = "metrics")]
    let t0 = std::time::Instant::now();
    #[cfg(feature = "metrics")]
    ensure_exporter();

    let result = (|| {
        // Check domain
        if c.domain != DOMAIN {
            return Err(SealError::BadDomain);
        }

        // Check scope
        if c.seal.scope != "capsule" {
            return Err(SealError::BadScope);
        }

        // Check audience
        if let Some(aud) = &c.seal.aud {
            match &c.hdr.dst {
                Some(dst) if dst == aud => {}
                _ => return Err(SealError::BadAudience),
            }
        }

        // Check expiration
        if let Some(exp) = c.hdr.exp {
            let now = opts.now_ns.unwrap_or_else(now_nanos_i64);
            if now.saturating_sub(opts.allowed_skew_ns) > exp {
                return Err(SealError::Expired { exp, now });
            }
        }

        // Check ID stability (Canon 3: ρ → encode → BLAKE3)
        let computed = compute_id(c)
            .map_err(|_| SealError::IdMismatch)?;
        if c.id != computed {
            return Err(SealError::IdMismatch);
        }

        // Verify Ed25519 signature
        let payload_hash = signing_hash(c)
            .map_err(|_| SealError::IdMismatch)?;
        let sig = ed25519_dalek::Signature::from_bytes(&c.seal.sig);
        use ed25519_dalek::Verifier;
        pk.verify(&payload_hash, &sig)
            .map_err(|_| SealError::BadSignature)
    })();

    #[cfg(feature = "metrics")]
    {
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        histogram!("capsule_seal_verify_ms").record(ms);
        match &result {
            Ok(()) => {
                counter!("capsule_seal_verify_ok_total").increment(1);
            }
            Err(e) => {
                counter!("capsule_seal_verify_fail_total", "code" => e.code()).increment(1);
            }
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Signing payload
// ---------------------------------------------------------------------------

/// The hash that gets signed: blake3(nrf.encode(ρ({domain, id, hdr, env})))
fn signing_hash(c: &Capsule) -> Result<[u8; 32], String> {
    let mut root = BTreeMap::new();
    root.insert("domain".into(), Value::String(c.domain.clone()));
    root.insert("id".into(), Value::Bytes(c.id.to_vec()));
    root.insert("hdr".into(), header_value(&c.hdr));
    root.insert("env".into(), envelope_value(&c.env)?);
    let normalized = nrf_core::rho::normalize(&Value::Map(root))
        .map_err(|e| format!("Err.Canon.Rho: {e}"))?;
    let nrf = nrf_core::encode(&normalized);
    Ok(*blake3::hash(&nrf).as_bytes())
}

fn header_value(h: &crate::types::Header) -> Value {
    let mut m = BTreeMap::new();
    m.insert("act".into(), Value::String(h.act.clone()));
    if let Some(dst) = &h.dst {
        m.insert("dst".into(), Value::String(dst.clone()));
    }
    m.insert("nonce".into(), Value::Bytes(h.nonce.to_vec()));
    if let Some(scope) = &h.scope {
        m.insert("scope".into(), Value::String(scope.clone()));
    }
    if let Some(exp) = h.exp {
        m.insert("exp".into(), Value::Int(exp));
    }
    m.insert("src".into(), Value::String(h.src.clone()));
    m.insert("ts".into(), Value::Int(h.ts));
    Value::Map(m)
}

fn envelope_value(e: &crate::types::Envelope) -> Result<Value, String> {
    let mut m = BTreeMap::new();
    // Canon 2,6: reject floats, never degrade
    m.insert("body".into(), json_to_nrf_strict(&e.body)?);
    if !e.evidence.is_empty() {
        m.insert(
            "evidence".into(),
            Value::Array(
                e.evidence
                    .iter()
                    .map(|s| Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    if let Some(links) = &e.links {
        let mut lm = BTreeMap::new();
        if let Some(prev) = &links.prev {
            lm.insert("prev".into(), Value::String(prev.clone()));
        }
        if !lm.is_empty() {
            m.insert("links".into(), Value::Map(lm));
        }
    }
    Ok(Value::Map(m))
}

/// Convert serde_json::Value to nrf_core::Value.
/// Canon 2: no floats. Canon 6: reject, never degrade.
fn json_to_nrf_strict(j: &serde_json::Value) -> Result<Value, String> {
    match j {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else {
                Err(format!("Err.Canon.Float: {n} is not Int64 — no floats, period"))
            }
        }
        serde_json::Value::String(s) => Ok(Value::String(s.clone())),
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(json_to_nrf_strict(item)?);
            }
            Ok(Value::Array(out))
        }
        serde_json::Value::Object(obj) => {
            let mut m = BTreeMap::new();
            for (k, v) in obj {
                m.insert(k.clone(), json_to_nrf_strict(v)?);
            }
            Ok(Value::Map(m))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
        let sk = ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn make_capsule() -> Capsule {
        Capsule {
            domain: DOMAIN.into(),
            id: [0u8; 32],
            hdr: Header {
                src: "did:ubl:alice#key-1".into(),
                dst: Some("did:ubl:bob".into()),
                nonce: [0xAA; 16],
                ts: 1700000000000,
                act: "ATTEST".into(),
                scope: None,
                exp: None,
            },
            env: Envelope {
                body: serde_json::json!({"name": "test", "value": 42}),
                links: None,
                evidence: vec![],
            },
            seal: Seal {
                kid: "did:ubl:alice#key-1".into(),
                sig: [0u8; 64],
                scope: "capsule".into(),
                aud: Some("did:ubl:bob".into()),
            },
            receipts: vec![],
        }
    }

    #[test]
    fn sign_and_verify_ok() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        assert_ne!(c.id, [0u8; 32]);
        assert_ne!(c.seal.sig, [0u8; 64]);
        assert!(verify(&c, &vk).is_ok());
    }

    #[test]
    fn tamper_env_fails() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        c.env.body = serde_json::json!({"tampered": true});
        // ID will mismatch because env changed
        assert_eq!(verify(&c, &vk).unwrap_err(), SealError::IdMismatch);
    }

    #[test]
    fn tamper_hdr_fails() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        c.hdr.act = "EVALUATE".into();
        assert_eq!(verify(&c, &vk).unwrap_err(), SealError::IdMismatch);
    }

    #[test]
    fn wrong_domain_fails() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        c.domain = "wrong/1.0".into();
        assert_eq!(verify(&c, &vk).unwrap_err(), SealError::BadDomain);
    }

    #[test]
    fn wrong_scope_fails() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        c.seal.scope = "wrong".into();
        assert_eq!(verify(&c, &vk).unwrap_err(), SealError::BadScope);
    }

    #[test]
    fn bad_audience_fails() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        c.seal.aud = Some("did:ubl:eve".into());
        assert_eq!(verify(&c, &vk).unwrap_err(), SealError::BadAudience);
    }

    #[test]
    fn no_aud_no_dst_ok() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        c.hdr.dst = None;
        c.seal.aud = None;
        sign(&mut c, &sk).unwrap();
        assert!(verify(&c, &vk).is_ok());
    }

    #[test]
    fn aud_present_dst_absent_fails() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        c.hdr.dst = None;
        c.seal.aud = Some("did:ubl:bob".into());
        sign(&mut c, &sk).unwrap();
        // Manually set aud after signing to bypass the sign flow
        assert_eq!(verify(&c, &vk).unwrap_err(), SealError::BadAudience);
    }

    #[test]
    fn wrong_key_fails() {
        let (sk, _vk) = keypair();
        let (_sk2, vk2) = keypair();
        let mut c = make_capsule();
        sign(&mut c, &sk).unwrap();
        assert_eq!(verify(&c, &vk2).unwrap_err(), SealError::BadSignature);
    }

    #[test]
    fn expired_capsule_rejected() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        c.hdr.exp = Some(1); // expired in 1970
        sign(&mut c, &sk).unwrap();
        assert!(matches!(
            verify(&c, &vk).unwrap_err(),
            SealError::Expired { .. }
        ));
    }

    #[test]
    fn future_exp_accepted() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        c.hdr.exp = Some(i64::MAX); // far future
        sign(&mut c, &sk).unwrap();
        assert!(verify(&c, &vk).is_ok());
    }

    #[test]
    fn expired_with_skew_accepted() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        let now = now_nanos_i64();
        c.hdr.exp = Some(now - 1_000_000_000); // 1s ago
        sign(&mut c, &sk).unwrap();
        assert!(verify(&c, &vk).is_err());
        let opts = VerifyOpts {
            allowed_skew_ns: 2_000_000_000,
            now_ns: None,
        };
        assert!(verify_with_opts(&c, &vk, &opts).is_ok());
    }

    #[test]
    fn no_exp_always_ok() {
        let (sk, vk) = keypair();
        let mut c = make_capsule();
        c.hdr.exp = None;
        sign(&mut c, &sk).unwrap();
        assert!(verify(&c, &vk).is_ok());
    }
}
