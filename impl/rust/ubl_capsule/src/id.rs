//! Capsule ID — stable content-address that does NOT change when
//! receipts or signatures are added/removed.
//!
//! `capsule_id = blake3(nrf.encode(capsule \ {id, seal.sig, receipts[*].sig}))`
//!
//! The excluded fields are exactly those that depend on the ID itself
//! (seal.sig signs over the ID) or are append-only metadata (receipt sigs).

use crate::types::Capsule;
use nrf_core::Value;
use std::collections::BTreeMap;

/// Compute the stable capsule ID.
///
/// The ID is `blake3(nrf.encode(ρ(core)))` where `core` is the capsule
/// with `id`, `seal.sig`, and all `receipts[*].sig` zeroed out.
///
/// Returns Err if env.body contains floats or non-i64 numbers.
/// Canon 2: no floats. Canon 3: ρ is the law. Canon 6: reject, never degrade.
pub fn compute_id(c: &Capsule) -> Result<[u8; 32], String> {
    let core = capsule_core_value(c)?;
    let normalized = nrf_core::rho::normalize(&core)
        .map_err(|e| format!("Err.Canon.Rho: {e}"))?;
    let bytes = nrf_core::encode(&normalized);
    Ok(*blake3::hash(&bytes).as_bytes())
}

/// Build the NRF Value representing the "core" of the capsule
/// (everything except id, seal.sig, and ALL receipts).
///
/// Receipts are entirely excluded because they are append-only metadata
/// that arrives after the capsule is created. The ID must be stable
/// regardless of how many hops are added.
fn capsule_core_value(c: &Capsule) -> Result<Value, String> {
    let mut root = BTreeMap::new();

    // domain
    root.insert("domain".into(), Value::String(c.domain.clone()));

    // hdr
    root.insert("hdr".into(), header_value(&c.hdr));

    // env — Canon 2,6: json_to_nrf_strict rejects floats
    root.insert("env".into(), envelope_value(&c.env)?);

    // seal (without sig — only kid, scope, aud)
    let mut seal_map = BTreeMap::new();
    if let Some(aud) = &c.seal.aud {
        seal_map.insert("aud".into(), Value::String(aud.clone()));
    }
    seal_map.insert("kid".into(), Value::String(c.seal.kid.clone()));
    seal_map.insert("scope".into(), Value::String(c.seal.scope.clone()));
    root.insert("seal".into(), Value::Map(seal_map));

    // receipts are EXCLUDED from ID computation

    Ok(Value::Map(root))
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
    // body: Canon 2,6 — reject floats, never degrade
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
    fn id_is_deterministic() {
        let c = make_capsule();
        let id1 = compute_id(&c).unwrap();
        let id2 = compute_id(&c).unwrap();
        assert_eq!(id1, id2);
        assert_ne!(id1, [0u8; 32]); // not all zeros
    }

    #[test]
    fn id_stable_after_adding_receipt() {
        let mut c = make_capsule();
        let id_before = compute_id(&c).unwrap();

        c.receipts.push(Receipt {
            id: [0xBB; 32],
            of: id_before,
            prev: [0u8; 32],
            kind: "relay".into(),
            node: "did:ubl:relay1#key-1".into(),
            ts: 1700000001000,
            sig: [0xCC; 64],
        });

        let id_after = compute_id(&c).unwrap();
        assert_eq!(
            id_before, id_after,
            "ID must NOT change when receipts are added"
        );
    }

    #[test]
    fn id_stable_after_removing_receipt() {
        let mut c = make_capsule();
        let id_base = compute_id(&c).unwrap();

        c.receipts.push(Receipt {
            id: [0xBB; 32],
            of: id_base,
            prev: [0u8; 32],
            kind: "relay".into(),
            node: "did:ubl:relay1#key-1".into(),
            ts: 1700000001000,
            sig: [0xCC; 64],
        });

        c.receipts.clear();
        let id_cleared = compute_id(&c).unwrap();
        assert_eq!(
            id_base, id_cleared,
            "ID must NOT change when receipts are removed"
        );
    }

    #[test]
    fn id_stable_regardless_of_seal_sig() {
        let mut c = make_capsule();
        let id1 = compute_id(&c).unwrap();
        c.seal.sig = [0xFF; 64];
        let id2 = compute_id(&c).unwrap();
        assert_eq!(id1, id2, "ID must NOT change when seal.sig changes");
    }

    #[test]
    fn id_changes_when_hdr_changes() {
        let c1 = make_capsule();
        let mut c2 = make_capsule();
        c2.hdr.act = "EVALUATE".into();
        assert_ne!(
            compute_id(&c1).unwrap(),
            compute_id(&c2).unwrap(),
            "different hdr → different ID"
        );
    }

    #[test]
    fn id_changes_when_env_changes() {
        let c1 = make_capsule();
        let mut c2 = make_capsule();
        c2.env.body = serde_json::json!({"name": "different"});
        assert_ne!(
            compute_id(&c1).unwrap(),
            compute_id(&c2).unwrap(),
            "different env → different ID"
        );
    }

    #[test]
    fn id_changes_when_domain_changes() {
        let c1 = make_capsule();
        let mut c2 = make_capsule();
        c2.domain = "ubl-capsule/2.0".into();
        assert_ne!(
            compute_id(&c1).unwrap(),
            compute_id(&c2).unwrap(),
            "different domain → different ID"
        );
    }

    #[test]
    fn empty_receipts_vs_none() {
        let c1 = make_capsule(); // receipts: vec![]
        let mut c2 = make_capsule();
        c2.receipts = vec![];
        assert_eq!(compute_id(&c1).unwrap(), compute_id(&c2).unwrap());
    }
}
