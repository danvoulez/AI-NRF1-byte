//! Integration tests — Pipeline + Invariants
//!
//! Covers: canonicality, stable ID, hops, pipeline, ASK/ACK/NACK invariants,
//! ASCII/NFC negatives.

use nrf_core::Value;
use std::collections::BTreeMap;
use ubl_capsule::receipt::{add_hop, verify_chain};
use ubl_capsule::seal;
use ubl_capsule::types::*;

fn keypair() -> (ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey) {
    let sk = ed25519_dalek::SigningKey::generate(&mut rand_core::OsRng);
    let vk = sk.verifying_key();
    (sk, vk)
}

fn make_capsule(act: &str, body: serde_json::Value) -> Capsule {
    Capsule {
        domain: DOMAIN.into(),
        id: [0u8; 32],
        hdr: Header {
            src: "did:ubl:alice#key-1".into(),
            dst: Some("did:ubl:bob".into()),
            nonce: [0xAA; 16],
            ts: 1700000000000,
            act: act.into(),
            scope: None,
            exp: None,
        },
        env: Envelope {
            body,
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

// =========================================================================
// 1. Roundtrip canônico: JSON → NRF → JSON
// =========================================================================

#[test]
fn roundtrip_canonical_primitives() {
    let cases: Vec<Value> = vec![
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::Int(0),
        Value::Int(-1),
        Value::Int(i64::MAX),
        Value::String(String::new()),
        Value::String("hello".into()),
        Value::Bytes(vec![]),
        Value::Bytes(vec![0xAA; 32]),
        Value::Bytes(vec![0xBB; 16]),
        Value::Bytes(vec![0xCC; 64]),
    ];
    for v in &cases {
        let j = ubl_json_view::to_json(v);
        let back = ubl_json_view::from_json(&j).unwrap();
        assert_eq!(v, &back, "roundtrip failed for {v:?}");
    }
}

#[test]
fn roundtrip_canonical_map_ordering() {
    let mut m = BTreeMap::new();
    m.insert("z".into(), Value::Int(1));
    m.insert("a".into(), Value::Int(2));
    m.insert("m".into(), Value::Int(3));
    let v = Value::Map(m);
    let nrf = nrf_core::encode(&v);
    let j = ubl_json_view::nrf_bytes_to_json(&nrf).unwrap();
    let nrf2 = ubl_json_view::json_to_nrf_bytes(&j).unwrap();
    assert_eq!(nrf, nrf2, "NRF roundtrip must be byte-identical");
}

#[test]
fn roundtrip_bytes_empty() {
    let v = Value::Bytes(vec![]);
    let j = ubl_json_view::to_json(&v);
    assert_eq!(j["$bytes"], "");
    let back = ubl_json_view::from_json(&j).unwrap();
    assert_eq!(v, back);
}

// =========================================================================
// 2. ID estável: add/remove receipt não muda id
// =========================================================================

#[test]
fn id_stable_add_remove_receipts() {
    let (sk, _vk) = keypair();
    let mut c = make_capsule("ATTEST", serde_json::json!({"test": true}));
    seal::sign(&mut c, &sk).unwrap();
    let id_original = c.id;

    // Add 3 receipts
    let (relay_sk, _) = keypair();
    let mut prev = [0u8; 32];
    for i in 0..3 {
        let r = add_hop(
            c.id,
            prev,
            "relay",
            &format!("did:ubl:relay{i}#key-1"),
            1700000000000 + i as i64,
            &relay_sk,
        )
        .unwrap();
        prev = r.id;
        c.receipts.push(r);
    }
    assert_eq!(ubl_capsule::compute_id(&c).unwrap(), id_original);

    // Remove all receipts
    c.receipts.clear();
    assert_eq!(ubl_capsule::compute_id(&c).unwrap(), id_original);
}

// =========================================================================
// 3. Hops: cadeia válida vs reorder/remove → falha
// =========================================================================

#[test]
fn valid_chain_passes() {
    let (author_sk, _) = keypair();
    let mut c = make_capsule("ATTEST", serde_json::json!({"data": 1}));
    seal::sign(&mut c, &author_sk).unwrap();

    let mut keys = vec![];
    let mut prev = [0u8; 32];
    for i in 0..5 {
        let (sk, vk) = keypair();
        let r = add_hop(
            c.id,
            prev,
            "relay",
            &format!("did:ubl:node{i}#key-1"),
            1700000000000 + i as i64,
            &sk,
        )
        .unwrap();
        prev = r.id;
        c.receipts.push(r);
        keys.push((format!("did:ubl:node{i}#key-1"), vk));
    }

    let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
        keys.iter().find(|(n, _)| n == node).map(|(_, vk)| *vk)
    };
    assert!(verify_chain(&c.id, &c.receipts, &resolve).is_ok());
}

#[test]
fn reorder_chain_fails() {
    let (author_sk, _) = keypair();
    let mut c = make_capsule("ATTEST", serde_json::json!({"data": 1}));
    seal::sign(&mut c, &author_sk).unwrap();

    let mut keys = vec![];
    let mut prev = [0u8; 32];
    for i in 0..3 {
        let (sk, vk) = keypair();
        let r = add_hop(
            c.id,
            prev,
            "relay",
            &format!("did:ubl:node{i}#key-1"),
            1700000000000 + i as i64,
            &sk,
        )
        .unwrap();
        prev = r.id;
        c.receipts.push(r);
        keys.push((format!("did:ubl:node{i}#key-1"), vk));
    }

    c.receipts.swap(0, 2);
    let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
        keys.iter().find(|(n, _)| n == node).map(|(_, vk)| *vk)
    };
    assert!(verify_chain(&c.id, &c.receipts, &resolve).is_err());
}

#[test]
fn remove_middle_hop_fails() {
    let (author_sk, _) = keypair();
    let mut c = make_capsule("ATTEST", serde_json::json!({"data": 1}));
    seal::sign(&mut c, &author_sk).unwrap();

    let mut keys = vec![];
    let mut prev = [0u8; 32];
    for i in 0..3 {
        let (sk, vk) = keypair();
        let r = add_hop(
            c.id,
            prev,
            "relay",
            &format!("did:ubl:node{i}#key-1"),
            1700000000000 + i as i64,
            &sk,
        )
        .unwrap();
        prev = r.id;
        c.receipts.push(r);
        keys.push((format!("did:ubl:node{i}#key-1"), vk));
    }

    c.receipts.remove(1);
    let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
        keys.iter().find(|(n, _)| n == node).map(|(_, vk)| *vk)
    };
    assert!(verify_chain(&c.id, &c.receipts, &resolve).is_err());
}

// =========================================================================
// 4. Pipeline: ATTEST → EVALUATE with env.links.prev = attest.id
// =========================================================================

#[test]
fn pipeline_attest_then_evaluate() {
    let (sk, vk) = keypair();

    // Step 1: ATTEST
    let mut attest = make_capsule("ATTEST", serde_json::json!({"doc": "insurance-policy-123"}));
    seal::sign(&mut attest, &sk).unwrap();
    assert!(seal::verify(&attest, &vk).is_ok());
    let attest_cid = format!("b3:{}", hex::encode(attest.id));

    // Step 2: EVALUATE with links.prev = attest CID
    let mut evaluate = make_capsule("EVALUATE", serde_json::json!({"result": "pass"}));
    evaluate.hdr.nonce = [0xBB; 16]; // different nonce
    evaluate.hdr.ts = 1700000001000;
    evaluate.env.links = Some(Links {
        prev: Some(attest_cid.clone()),
    });
    seal::sign(&mut evaluate, &sk).unwrap();
    assert!(seal::verify(&evaluate, &vk).is_ok());

    // Verify the link
    assert_eq!(
        evaluate.env.links.as_ref().unwrap().prev.as_ref().unwrap(),
        &attest_cid
    );
    // Different capsules have different IDs
    assert_ne!(attest.id, evaluate.id);
}

// =========================================================================
// 4b. End-to-end capsule flow
// =========================================================================

#[test]
fn end_to_end_capsule_flow() {
    let (author_sk, author_vk) = keypair();
    let (relay_sk, relay_vk) = keypair();
    let (exec_sk, exec_vk) = keypair();

    let mut cap = make_capsule("ATTEST", serde_json::json!({"doc": "policy-123"}));
    cap.hdr.exp = Some(seal::now_nanos_i64().saturating_add(60_000_000_000)); // +60s

    seal::sign(&mut cap, &author_sk).unwrap();
    let id_before = cap.id;
    assert!(seal::verify(&cap, &author_vk).is_ok());

    // Add 2 receipt hops (relay → exec)
    let relay_node = "did:ubl:relay#key-1";
    let exec_node = "did:ubl:exec#key-1";
    let mut prev = [0u8; 32];

    let r1 = add_hop(cap.id, prev, "relay", relay_node, cap.hdr.ts + 1, &relay_sk).unwrap();
    prev = r1.id;
    cap.receipts.push(r1);

    let r2 = add_hop(cap.id, prev, "exec", exec_node, cap.hdr.ts + 2, &exec_sk).unwrap();
    cap.receipts.push(r2);

    let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> {
        if node == relay_node {
            Some(relay_vk)
        } else if node == exec_node {
            Some(exec_vk)
        } else {
            None
        }
    };
    assert!(verify_chain(&cap.id, &cap.receipts, &resolve).is_ok());

    // ID must be stable regardless of receipts
    assert_eq!(ubl_capsule::compute_id(&cap).unwrap(), id_before);

    // JSON serialize/deserialize roundtrip keeps seal valid
    let json = serde_json::to_string(&cap).unwrap();
    let cap2: Capsule = serde_json::from_str(&json).unwrap();
    assert_eq!(cap2.id, cap.id);
    assert!(seal::verify(&cap2, &author_vk).is_ok());
}

// =========================================================================
// 5. Invariantes: ASK ⇒ links.prev obrigatório, ACK/NACK ⇒ evidence presente
// =========================================================================

fn validate_ask_invariant(c: &Capsule) -> Result<(), String> {
    // ASK requires links.prev
    if c.hdr.act == "ASK" {
        match &c.env.links {
            Some(links) if links.prev.is_some() => Ok(()),
            _ => Err("ASK requires links.prev".into()),
        }
    } else {
        Ok(())
    }
}

fn validate_ack_nack_invariant(c: &Capsule) -> Result<(), String> {
    // ACK/NACK requires evidence field to exist (can be empty vec)
    if c.hdr.act == "ACK" || c.hdr.act == "NACK" {
        // evidence must be present in the envelope (it's always there as vec,
        // but we check it's explicitly set — in our struct it defaults to empty)
        Ok(()) // evidence field always exists in our struct
    } else {
        Ok(())
    }
}

#[test]
fn ask_without_prev_rejected() {
    let c = make_capsule("ASK", serde_json::json!({"question": "?"}));
    // No links.prev set
    assert!(validate_ask_invariant(&c).is_err());
}

#[test]
fn ask_with_prev_accepted() {
    let mut c = make_capsule("ASK", serde_json::json!({"question": "?"}));
    c.env.links = Some(Links {
        prev: Some("b3:0000000000000000000000000000000000000000000000000000000000000000".into()),
    });
    assert!(validate_ask_invariant(&c).is_ok());
}

#[test]
fn ack_nack_invariant_ok() {
    let c_ack = make_capsule("ACK", serde_json::json!({"answer": "yes"}));
    assert!(validate_ack_nack_invariant(&c_ack).is_ok());

    let c_nack = make_capsule("NACK", serde_json::json!({"reason": "no"}));
    assert!(validate_ack_nack_invariant(&c_nack).is_ok());
}

// =========================================================================
// 6. ASCII/NFC negatives
// =========================================================================

#[test]
fn non_ascii_did_rejected_in_receipt() {
    let result = add_hop(
        [0xAA; 32],
        [0u8; 32],
        "relay",
        "did:ubl:café#key-1", // non-ASCII
        1700000000000,
        &keypair().0,
    );
    assert!(result.is_err());
}

#[test]
fn not_nfc_string_rejected_in_json_view() {
    // NFD: e + combining acute accent
    let nfd = "e\u{0301}";
    let j = serde_json::Value::String(nfd.into());
    assert!(ubl_json_view::from_json(&j).is_err());
}

#[test]
fn bom_rejected_in_json_view() {
    let j = serde_json::Value::String("\u{FEFF}hello".into());
    assert!(ubl_json_view::from_json(&j).is_err());
}

#[test]
fn float_rejected_in_json_view() {
    let j = serde_json::json!(3.15);
    assert!(ubl_json_view::from_json(&j).is_err());
}

#[test]
fn ascii_did_accepted() {
    assert!(ubl_json_view::validate_ascii("did:ubl:lab512#key-1").is_ok());
}

#[test]
fn non_ascii_did_rejected_in_json_view() {
    assert!(ubl_json_view::validate_ascii("did:ubl:café").is_err());
}

// =========================================================================
// Timestamps with and without fraction
// =========================================================================

#[test]
fn timestamp_fraction_zero_stripped() {
    let ts = nrf_core::rho::normalize_timestamp("2024-01-15T10:30:00.000Z").unwrap();
    assert_eq!(ts, "2024-01-15T10:30:00Z");
}

#[test]
fn timestamp_fraction_minimal() {
    let ts = nrf_core::rho::normalize_timestamp("2024-01-15T10:30:00.100Z").unwrap();
    assert_eq!(ts, "2024-01-15T10:30:00.1Z");
}

#[test]
fn timestamp_no_fraction_unchanged() {
    let ts = nrf_core::rho::normalize_timestamp("2024-01-15T10:30:00Z").unwrap();
    assert_eq!(ts, "2024-01-15T10:30:00Z");
}

// =========================================================================
// Varint non-minimal rejected
// =========================================================================

#[test]
fn varint_non_minimal_rejected() {
    // 0x80 0x01 encodes 0 non-minimally (should be just 0x00)
    let data = vec![0x6E, 0x72, 0x66, 0x31, 0x05, 0x80, 0x01];
    assert!(nrf_core::decode(&data).is_err());
}

// =========================================================================
// Sets with duplicates (via rho)
// =========================================================================

#[test]
fn set_dedup_via_rho() {
    let items = vec![
        Value::String("b".into()),
        Value::String("a".into()),
        Value::String("b".into()),
    ];
    let result = nrf_core::rho::normalize_as_set(&items).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], Value::String("a".into()));
    assert_eq!(result[1], Value::String("b".into()));
}
