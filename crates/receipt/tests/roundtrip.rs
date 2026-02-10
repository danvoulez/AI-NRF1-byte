use ed25519_dalek::SigningKey;
use nrf1::Value;
use receipt::*;
use std::collections::BTreeMap;

fn make_test_receipt() -> Receipt {
    let body = Value::Map({
        let mut m = BTreeMap::new();
        m.insert("hello".into(), Value::String("world".into()));
        m
    });
    let body_cid = nrf1::blake3_cid(&body);

    Receipt {
        v: "receipt-v1".into(),
        receipt_cid: String::new(), // will be computed
        t: 1_700_000_000_000_000_000,
        issuer_did: "did:ubl:test-issuer".into(),
        subject_did: None,
        kid: None,
        act: "ATTEST".into(),
        subject: "b3:0000000000000000000000000000000000000000000000000000000000000000".into(),
        decision: Some("ALLOW".into()),
        effects: None,
        body,
        body_cid,
        inputs_cid: None,
        policy: None,
        reasoning_cid: None,
        permit_cid: None,
        pipeline_prev: vec![],
        rt: RuntimeInfo {
            name: "test-runtime".into(),
            version: "0.1.0".into(),
            binary_sha256: "abcd1234".into(),
            hal_ref: None,
            env: BTreeMap::new(),
            certs: vec![],
        },
        prev: None,
        chain: None,
        ghost: None,
        nonce: vec![0u8; 16],
        url: "https://example.com/receipts/test.json".into(),
        sig: None,
    }
}

#[test]
fn test_body_cid_consistency() {
    let r = make_test_receipt();
    assert_eq!(r.compute_body_cid(), r.body_cid);
}

#[test]
fn test_receipt_cid_deterministic() {
    let mut r = make_test_receipt();
    r.receipt_cid = r.compute_cid();
    let cid1 = r.compute_cid();
    let cid2 = r.compute_cid();
    assert_eq!(cid1, cid2);
    assert!(cid1.starts_with("b3:"));
}

#[test]
fn test_sign_and_verify() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();

    let mut r = make_test_receipt();
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);

    assert!(r.sig.is_some());
    assert!(r.verify(&vk));
}

#[test]
fn test_verify_fails_with_wrong_key() {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let wrong_sk = SigningKey::generate(&mut rng);
    let wrong_vk = wrong_sk.verifying_key();

    let mut r = make_test_receipt();
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);

    assert!(!r.verify(&wrong_vk));
}

#[test]
fn test_verify_integrity() {
    let mut r = make_test_receipt();
    r.receipt_cid = r.compute_cid();
    assert!(r.verify_integrity().is_ok());
}

#[test]
fn test_ghost_invariant_gho001() {
    let mut r = make_test_receipt();
    r.decision = Some("GHOST".into());
    r.effects = Some(Value::Null); // effects present = violation
    r.receipt_cid = r.compute_cid();
    assert!(r.verify_integrity().is_err());
}

#[test]
fn test_ghost_decision_null_effects_ok() {
    let mut r = make_test_receipt();
    r.decision = Some("GHOST".into());
    r.effects = None;
    r.receipt_cid = r.compute_cid();
    assert!(r.verify_integrity().is_ok());
}
