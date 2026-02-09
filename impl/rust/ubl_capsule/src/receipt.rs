//! Receipt hops — append-only SIRP custody chain.
//!
//! Each receipt proves a hop in the delivery/execution path:
//!   `{domain:"ubl-receipt/1.0", of, prev, kind, node, ts}`
//!
//! `receipt_id = blake3(nrf.encode(receipt_payload))`
//! The `receipt_id` becomes the next hop's `prev`.
//!
//! Chain verification: prev links must form a contiguous chain
//! starting from `prev = 0x00…00` (first hop).

use crate::types::{Receipt, RECEIPT_DOMAIN};
use nrf_core::Value;
use std::collections::BTreeMap;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HopError {
    #[error("Err.Hop.BadChain: receipt[{0}].prev does not match previous receipt ID")]
    BadChain(usize),
    #[error("Err.Hop.BadSignature: receipt[{0}] signature invalid")]
    BadSignature(usize),
    #[error("Err.Hop.BadDomain")]
    BadDomain,
    #[error("Err.Hop.NotASCII: node must be ASCII")]
    NotASCII,
    #[error("Err.Hop.Fork: duplicate prev detected at receipt[{0}]")]
    Fork(usize),
}

// ---------------------------------------------------------------------------
// Receipt payload → ID
// ---------------------------------------------------------------------------

/// Build the canonical NRF payload for a receipt (without sig).
pub fn receipt_payload(r: &Receipt) -> Value {
    let mut m = BTreeMap::new();
    m.insert("domain".into(), Value::String(RECEIPT_DOMAIN.into()));
    m.insert("kind".into(), Value::String(r.kind.clone()));
    m.insert("node".into(), Value::String(r.node.clone()));
    m.insert("of".into(), Value::Bytes(r.of.to_vec()));
    m.insert("prev".into(), Value::Bytes(r.prev.to_vec()));
    m.insert("ts".into(), Value::Int(r.ts));
    Value::Map(m)
}

/// Compute the receipt ID: `blake3(nrf.encode(receipt_payload))`.
pub fn compute_receipt_id(r: &Receipt) -> [u8; 32] {
    let payload = receipt_payload(r);
    let bytes = nrf_core::encode(&payload);
    *blake3::hash(&bytes).as_bytes()
}

// ---------------------------------------------------------------------------
// Sign / Verify a single receipt
// ---------------------------------------------------------------------------

/// Sign a receipt: compute its ID and produce an Ed25519 signature over
/// `blake3(nrf.encode(receipt_payload))`.
pub fn sign_receipt(r: &mut Receipt, sk: &ed25519_dalek::SigningKey) {
    let payload = receipt_payload(r);
    let bytes = nrf_core::encode(&payload);
    let hash = blake3::hash(&bytes);
    r.id = *hash.as_bytes();

    use ed25519_dalek::Signer;
    let sig = sk.sign(hash.as_bytes());
    r.sig = sig.to_bytes();
}

/// Verify a single receipt's signature.
pub fn verify_receipt(r: &Receipt, pk: &ed25519_dalek::VerifyingKey) -> Result<(), HopError> {
    // Verify ID matches payload
    let expected_id = compute_receipt_id(r);
    if r.id != expected_id {
        return Err(HopError::BadChain(0));
    }

    // Verify node is ASCII
    if !r.node.is_ascii() {
        return Err(HopError::NotASCII);
    }

    // Verify signature
    let sig = ed25519_dalek::Signature::from_bytes(&r.sig);
    use ed25519_dalek::Verifier;
    pk.verify(&r.id, &sig).map_err(|_| HopError::BadSignature(0))
}

// ---------------------------------------------------------------------------
// Chain verification
// ---------------------------------------------------------------------------

/// Verify the entire receipt chain for a capsule.
///
/// Rules:
///   1. First receipt's `prev` must be `[0u8; 32]` (genesis)
///   2. Each subsequent receipt's `prev` must equal the previous receipt's `id`
///   3. All `of` fields must equal `capsule_id`
///   4. All signatures must be valid (caller provides verifier fn)
///   5. No forks (no duplicate `prev` values)
pub fn verify_chain(
    capsule_id: &[u8; 32],
    receipts: &[Receipt],
    resolve_pk: &dyn Fn(&str) -> Option<ed25519_dalek::VerifyingKey>,
) -> Result<(), HopError> {
    let mut expected_prev = [0u8; 32]; // genesis
    let mut seen_prevs = std::collections::HashSet::new();

    for (i, r) in receipts.iter().enumerate() {
        // Check prev chain
        if r.prev != expected_prev {
            return Err(HopError::BadChain(i));
        }

        // Check for fork (duplicate prev)
        if !seen_prevs.insert(r.prev) {
            return Err(HopError::Fork(i));
        }

        // Check `of` matches capsule_id
        if r.of != *capsule_id {
            return Err(HopError::BadChain(i));
        }

        // Check node is ASCII
        if !r.node.is_ascii() {
            return Err(HopError::NotASCII);
        }

        // Verify receipt ID
        let expected_id = compute_receipt_id(r);
        if r.id != expected_id {
            return Err(HopError::BadChain(i));
        }

        // Verify signature
        let pk = resolve_pk(&r.node).ok_or(HopError::BadSignature(i))?;
        let sig = ed25519_dalek::Signature::from_bytes(&r.sig);
        use ed25519_dalek::Verifier;
        pk.verify(&r.id, &sig).map_err(|_| HopError::BadSignature(i))?;

        // Next hop's prev = this receipt's id
        expected_prev = r.id;
    }

    Ok(())
}

/// Create a new receipt hop and sign it.
pub fn add_hop(
    capsule_id: [u8; 32],
    prev: [u8; 32],
    kind: &str,
    node: &str,
    ts: i64,
    sk: &ed25519_dalek::SigningKey,
) -> Result<Receipt, HopError> {
    if !node.is_ascii() {
        return Err(HopError::NotASCII);
    }
    let mut r = Receipt {
        id: [0u8; 32],
        of: capsule_id,
        prev,
        kind: kind.into(),
        node: node.into(),
        ts,
        sig: [0u8; 64],
    };
    sign_receipt(&mut r, sk);
    Ok(r)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
        let sk = ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn build_chain(
        capsule_id: [u8; 32],
        n: usize,
    ) -> (Vec<Receipt>, Vec<(ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey)>) {
        let mut receipts = Vec::new();
        let mut keys = Vec::new();
        let mut prev = [0u8; 32];
        for i in 0..n {
            let (sk, vk) = keypair();
            let node = format!("did:ubl:node{i}#key-1");
            let r = add_hop(capsule_id, prev, "relay", &node, 1700000000000 + i as i64, &sk)
                .unwrap();
            prev = r.id;
            receipts.push(r);
            keys.push((sk, vk));
        }
        (receipts, keys)
    }

    #[test]
    fn valid_chain_3_hops() {
        let capsule_id = [0xAA; 32];
        let (receipts, keys) = build_chain(capsule_id, 3);
        let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
            for (i, r) in receipts.iter().enumerate() {
                if r.node == node {
                    return Some(keys[i].1);
                }
            }
            None
        };
        assert!(verify_chain(&capsule_id, &receipts, &resolve).is_ok());
    }

    #[test]
    fn valid_chain_10_hops() {
        let capsule_id = [0xBB; 32];
        let (receipts, keys) = build_chain(capsule_id, 10);
        let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
            for (i, r) in receipts.iter().enumerate() {
                if r.node == node {
                    return Some(keys[i].1);
                }
            }
            None
        };
        assert!(verify_chain(&capsule_id, &receipts, &resolve).is_ok());
    }

    #[test]
    fn reorder_fails() {
        let capsule_id = [0xCC; 32];
        let (mut receipts, keys) = build_chain(capsule_id, 3);
        receipts.swap(1, 2);
        let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
            for (i, r) in receipts.iter().enumerate() {
                if r.node == node {
                    return Some(keys[i].1);
                }
            }
            None
        };
        assert!(verify_chain(&capsule_id, &receipts, &resolve).is_err());
    }

    #[test]
    fn remove_hop_fails() {
        let capsule_id = [0xDD; 32];
        let (mut receipts, keys) = build_chain(capsule_id, 3);
        receipts.remove(1); // remove middle hop
        let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
            for (i, r) in receipts.iter().enumerate() {
                if r.node == node {
                    return Some(keys[i].1);
                }
            }
            None
        };
        assert!(verify_chain(&capsule_id, &receipts, &resolve).is_err());
    }

    #[test]
    fn first_hop_prev_must_be_zero() {
        let capsule_id = [0xEE; 32];
        let (sk, vk) = keypair();
        let mut r = Receipt {
            id: [0u8; 32],
            of: capsule_id,
            prev: [0xFF; 32], // NOT zero
            kind: "relay".into(),
            node: "did:ubl:node0#key-1".into(),
            ts: 1700000000000,
            sig: [0u8; 64],
        };
        sign_receipt(&mut r, &sk);
        let resolve = |_node: &str| -> Option<ed25519_dalek::VerifyingKey> { Some(vk) };
        assert!(verify_chain(&capsule_id, &[r], &resolve).is_err());
    }

    #[test]
    fn non_ascii_node_rejected() {
        let result = add_hop(
            [0xAA; 32],
            [0u8; 32],
            "relay",
            "did:ubl:café#key-1",
            1700000000000,
            &keypair().0,
        );
        assert_eq!(result.unwrap_err(), HopError::NotASCII);
    }

    #[test]
    fn empty_chain_ok() {
        let capsule_id = [0xFF; 32];
        let resolve = |_: &str| -> Option<ed25519_dalek::VerifyingKey> { None };
        assert!(verify_chain(&capsule_id, &[], &resolve).is_ok());
    }

    #[test]
    fn receipt_id_deterministic() {
        let (sk, _vk) = keypair();
        let mut r1 = Receipt {
            id: [0u8; 32],
            of: [0xAA; 32],
            prev: [0u8; 32],
            kind: "relay".into(),
            node: "did:ubl:node0#key-1".into(),
            ts: 1700000000000,
            sig: [0u8; 64],
        };
        sign_receipt(&mut r1, &sk);

        let mut r2 = Receipt {
            id: [0u8; 32],
            of: [0xAA; 32],
            prev: [0u8; 32],
            kind: "relay".into(),
            node: "did:ubl:node0#key-1".into(),
            ts: 1700000000000,
            sig: [0u8; 64],
        };
        sign_receipt(&mut r2, &sk);

        assert_eq!(r1.id, r2.id);
    }
}
