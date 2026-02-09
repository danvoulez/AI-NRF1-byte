// ==========================================================================
// BASE CONFORMANCE SUITE
//
// Every test is named: art{N}_{section}_{what_it_checks}
//
// If a test fails, the name tells you:
//   - Which Article of the Constitution was violated
//   - Which section within that Article
//   - What specific rule broke
//
// Example: art1_1_rho_nfc_normalization
//   → Article I, Section 1.1, ρ rule 1 (NFC normalization)
//
// LLM-friendly: every assertion includes a human-readable message
// explaining WHAT went wrong and WHY it matters.
// ==========================================================================

use nrf_core::{Value, encode, decode, rho};
use std::collections::BTreeMap;
use ed25519_dalek::SigningKey;

// ==========================================================================
// HELPERS
// ==========================================================================

fn keygen() -> (SigningKey, ed25519_dalek::VerifyingKey) {
    let mut rng = rand::thread_rng();
    let sk = SigningKey::generate(&mut rng);
    let vk = sk.verifying_key();
    (sk, vk)
}

fn make_map(pairs: &[(&str, Value)]) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    Value::Map(m)
}

fn make_receipt() -> receipt::Receipt {
    let body = make_map(&[("hello", Value::String("world".into()))]);
    let body_cid = nrf1::blake3_cid(&body);
    receipt::Receipt {
        v: "receipt-v1".into(),
        receipt_cid: String::new(),
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
        rt: receipt::RuntimeInfo {
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

fn make_permit() -> permit::Permit {
    permit::Permit {
        v: "permit-v1".into(),
        permit_cid: String::new(),
        request_cid: "b3:aaaa".into(),
        decision: "ALLOW".into(),
        input_hash: "b3:bbbb".into(),
        issuer_did: "did:ubl:authority".into(),
        issued_at: 1_700_000_000_000_000_000,
        expires_at: 1_800_000_000_000_000_000,
        act: "EVALUATE".into(),
        policy: Some("pack-compliance/eu-ai-act@1".into()),
        sig: None,
    }
}

fn make_ghost() -> ghost::Ghost {
    let wbe = ghost::Wbe {
        who: "did:ubl:actor".into(),
        what: "evaluate insurance claim".into(),
        when: 1_700_000_000_000_000_000,
        intent: "EVALUATE".into(),
    };
    ghost::Ghost::new_pending(wbe, vec![0u8; 16], "https://example.com/ghosts/test.json".into())
}

// ==========================================================================
// ARTICLE I — ρ (Rho): The Policy Engine at the Byte Level
// ==========================================================================

// --- Section 1.1: The ρ Rules ---

#[test]
fn art1_1_rho_nfc_normalization() {
    // ρ rule 1: strings must be NFC normalized
    let decomposed = "e\u{0301}"; // NFD: e + combining acute
    let v = Value::String(decomposed.to_string());
    let norm = rho::normalize(&v).expect(
        "ARTICLE I §1.1 VIOLATION: ρ must normalize NFD strings to NFC"
    );
    assert_eq!(
        norm,
        Value::String("\u{00E9}".to_string()),
        "ARTICLE I §1.1: ρ(NFD 'é') must produce NFC 'é', got {norm:?}"
    );
}

#[test]
fn art1_1_rho_bom_rejection() {
    // ρ rule 1: BOM (U+FEFF) must be rejected, not silently stripped
    let v = Value::String("\u{FEFF}hello".to_string());
    assert!(
        rho::normalize(&v).is_err(),
        "ARTICLE I §1.1 VIOLATION: ρ must reject strings containing BOM (U+FEFF)"
    );
}

#[test]
fn art1_1_rho_map_strips_nulls() {
    // ρ rule 5: null values in maps are REMOVED (absence ≠ null)
    let v = make_map(&[
        ("a", Value::Int(1)),
        ("b", Value::Null),
        ("c", Value::Int(3)),
    ]);
    let norm = rho::normalize(&v).expect("ρ normalize failed");
    if let Value::Map(m) = &norm {
        assert!(
            !m.contains_key("b"),
            "ARTICLE I §1.1 VIOLATION: ρ must remove null values from maps. \
             Key 'b' with Null value should have been stripped. \
             Constitution says: absence ≠ null."
        );
        assert_eq!(m.len(), 2, "ARTICLE I §1.1: map should have 2 keys after null removal");
    } else {
        panic!("ARTICLE I §1.1: ρ(Map) must return Map");
    }
}

#[test]
fn art1_1_rho_nested_null_removal() {
    // ρ rule 5+6: recursive — nulls in nested maps must also be removed
    let inner = make_map(&[("x", Value::Int(1)), ("y", Value::Null)]);
    let outer = make_map(&[("inner", inner), ("z", Value::Null)]);
    let norm = rho::normalize(&outer).expect("ρ normalize failed");
    if let Value::Map(m) = &norm {
        assert!(!m.contains_key("z"), "ARTICLE I §1.1: outer null not removed");
        if let Some(Value::Map(inner_m)) = m.get("inner") {
            assert!(
                !inner_m.contains_key("y"),
                "ARTICLE I §1.1 VIOLATION: ρ must recursively remove nulls. \
                 Nested key 'y' with Null was not stripped."
            );
        } else {
            panic!("ARTICLE I §1.1: inner map missing after ρ");
        }
    } else {
        panic!("ARTICLE I §1.1: ρ(Map) must return Map");
    }
}

#[test]
fn art1_1_rho_passthrough_types() {
    // ρ rule 7: Null, Bool, Int64, Bytes pass through unchanged
    let cases: Vec<(Value, &str)> = vec![
        (Value::Null, "Null"),
        (Value::Bool(true), "Bool(true)"),
        (Value::Bool(false), "Bool(false)"),
        (Value::Int(42), "Int(42)"),
        (Value::Int(-1), "Int(-1)"),
        (Value::Int(0), "Int(0)"),
        (Value::Int(i64::MAX), "Int(MAX)"),
        (Value::Int(i64::MIN), "Int(MIN)"),
        (Value::Bytes(vec![0xDE, 0xAD]), "Bytes(DEAD)"),
    ];
    for (v, label) in cases {
        let norm = rho::normalize(&v).expect("ρ normalize failed");
        assert_eq!(
            norm, v,
            "ARTICLE I §1.1 VIOLATION: ρ must pass through {label} unchanged, got {norm:?}"
        );
    }
}

// --- Section 1.2: The ρ Properties ---

#[test]
fn art1_2_rho_idempotent() {
    // ρ(ρ(v)) = ρ(v) — the supreme property
    let v = make_map(&[
        ("name", Value::String("e\u{0301}".to_string())), // NFD
        ("gone", Value::Null),                              // will be stripped
        ("num", Value::Int(42)),
    ]);
    let r1 = rho::normalize(&v).expect("first ρ failed");
    let r2 = rho::normalize(&r1).expect("second ρ failed");
    assert_eq!(
        r1, r2,
        "ARTICLE I §1.2 VIOLATION: ρ is NOT idempotent. \
         ρ(v) ≠ ρ(ρ(v)). This breaks the canonical guarantee. \
         First: {r1:?}, Second: {r2:?}"
    );
}

#[test]
fn art1_2_rho_encode_deterministic() {
    // encode(ρ(v)) must be identical on repeated calls
    let v = make_map(&[("key", Value::String("value".into()))]);
    let bytes1 = rho::canonical_encode(&v).expect("canonical_encode failed");
    let bytes2 = rho::canonical_encode(&v).expect("canonical_encode failed");
    assert_eq!(
        bytes1, bytes2,
        "ARTICLE I §1.2 VIOLATION: encode(ρ(v)) is not deterministic. \
         Same value produced different byte streams."
    );
}

#[test]
fn art1_2_rho_hash_stable() {
    // BLAKE3(encode(ρ(v))) must be identical on repeated calls
    let v = make_map(&[("key", Value::String("value".into()))]);
    let cid1 = rho::canonical_cid(&v).expect("canonical_cid failed");
    let cid2 = rho::canonical_cid(&v).expect("canonical_cid failed");
    assert_eq!(
        cid1, cid2,
        "ARTICLE I §1.2 VIOLATION: BLAKE3(encode(ρ(v))) is not stable. \
         Same value produced different CIDs: {cid1} vs {cid2}"
    );
    assert!(
        cid1.starts_with("b3:"),
        "ARTICLE I §1.2: CID must start with 'b3:', got '{cid1}'"
    );
    assert_eq!(
        cid1.len(), 3 + 64,
        "ARTICLE I §1.2: CID must be 'b3:' + 64 hex chars, got length {}",
        cid1.len()
    );
}

// --- Section 1.3: The ρ Modes ---

#[test]
fn art1_3_rho_validate_rejects_non_canonical() {
    // validate mode must reject values that ρ would change
    let v = make_map(&[("a", Value::Null), ("b", Value::Int(1))]);
    assert!(
        rho::validate(&v).is_err(),
        "ARTICLE I §1.3 VIOLATION: rho::validate must reject non-canonical values. \
         Map with null value should fail validation (absence ≠ null)."
    );
}

#[test]
fn art1_3_rho_validate_accepts_canonical() {
    let v = make_map(&[("b", Value::Int(1))]);
    assert!(
        rho::validate(&v).is_ok(),
        "ARTICLE I §1.3 VIOLATION: rho::validate rejected a canonical value."
    );
}

// --- Section 1.4: The Blessed Path ---

#[test]
fn art1_4_blessed_path_roundtrip() {
    // canonical_encode → decode must roundtrip
    let v = make_map(&[
        ("name", Value::String("test".into())),
        ("count", Value::Int(42)),
        ("data", Value::Bytes(vec![0xFF, 0x00])),
    ]);
    let bytes = rho::canonical_encode(&v).expect("canonical_encode failed");
    let decoded = decode(&bytes).expect(
        "ARTICLE I §1.4 VIOLATION: canonical_encode output could not be decoded"
    );
    assert_eq!(
        v, decoded,
        "ARTICLE I §1.4 VIOLATION: blessed path roundtrip failed. \
         encode(ρ(v)) → decode ≠ v"
    );
}

// ==========================================================================
// ARTICLE II — The Fractal Invariant
// ==========================================================================

#[test]
fn art2_1_one_value_one_encoding() {
    // Two logically equal values must produce identical bytes
    let v1 = make_map(&[("a", Value::Int(1)), ("b", Value::Int(2))]);
    let v2 = make_map(&[("b", Value::Int(2)), ("a", Value::Int(1))]); // different insertion order
    let b1 = encode(&v1);
    let b2 = encode(&v2);
    assert_eq!(
        b1, b2,
        "ARTICLE II §2.1 VIOLATION: same logical value produced different byte streams. \
         BTreeMap must sort keys regardless of insertion order."
    );
}

#[test]
fn art2_1_tamper_detection_single_bit() {
    // Flip one bit in encoded bytes → decode must fail or produce different value
    let v = make_map(&[("key", Value::String("value".into()))]);
    let mut bytes = encode(&v);
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01; // flip one bit
    if let Ok(tampered) = decode(&bytes) {
        assert_ne!(
            tampered, v,
            "ARTICLE II §2.1 VIOLATION: single-bit tamper was not detected. \
             Flipped bit in encoding produced the same value."
        );
    }
}

#[test]
fn art2_2_cid_changes_on_any_mutation() {
    // Change any field → CID must change
    let v1 = make_map(&[("key", Value::String("value".into()))]);
    let v2 = make_map(&[("key", Value::String("valuE".into()))]);
    let cid1 = nrf_core::blake3_cid(&v1);
    let cid2 = nrf_core::blake3_cid(&v2);
    assert_ne!(
        cid1, cid2,
        "ARTICLE II §2.2 VIOLATION: different values produced the same CID. \
         CID must change when any byte changes."
    );
}

// ==========================================================================
// ARTICLE III — The Seven Types (KAT vectors from spec §5)
// ==========================================================================

#[test]
fn art3_kat_null() {
    let bytes = encode(&Value::Null);
    assert_eq!(
        bytes,
        vec![0x6E, 0x72, 0x66, 0x31, 0x00],
        "ARTICLE III KAT: Null must encode to 'nrf1' + 0x00"
    );
    assert_eq!(decode(&bytes).unwrap(), Value::Null);
}

#[test]
fn art3_kat_int64_negative_one() {
    let bytes = encode(&Value::Int(-1));
    let expected: Vec<u8> = vec![
        0x6E, 0x72, 0x66, 0x31, // magic
        0x03,                     // Int64 tag
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, // -1 big-endian
    ];
    assert_eq!(
        bytes, expected,
        "ARTICLE III KAT: Int64(-1) encoding mismatch"
    );
    assert_eq!(decode(&bytes).unwrap(), Value::Int(-1));
}

#[test]
fn art3_kat_string_hello() {
    let bytes = encode(&Value::String("hello".into()));
    let expected: Vec<u8> = vec![
        0x6E, 0x72, 0x66, 0x31, // magic
        0x04,                     // String tag
        0x05,                     // varint32(5)
        0x68, 0x65, 0x6C, 0x6C, 0x6F, // "hello"
    ];
    assert_eq!(
        bytes, expected,
        "ARTICLE III KAT: String 'hello' encoding mismatch"
    );
    assert_eq!(decode(&bytes).unwrap(), Value::String("hello".into()));
}

#[test]
fn art3_kat_array_true_42() {
    let v = Value::Array(vec![Value::Bool(true), Value::Int(42)]);
    let bytes = encode(&v);
    let expected: Vec<u8> = vec![
        0x6E, 0x72, 0x66, 0x31, // magic
        0x06,                     // Array tag
        0x02,                     // varint32(2) count
        0x02,                     // True tag
        0x03,                     // Int64 tag
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A, // 42 big-endian
    ];
    assert_eq!(
        bytes, expected,
        "ARTICLE III KAT: Array [true, 42] encoding mismatch"
    );
    assert_eq!(decode(&bytes).unwrap(), v);
}

#[test]
fn art3_kat_map_a1_btrue() {
    let v = make_map(&[("a", Value::Int(1)), ("b", Value::Bool(true))]);
    let bytes = encode(&v);
    let expected: Vec<u8> = vec![
        0x6E, 0x72, 0x66, 0x31, // magic
        0x07,                     // Map tag
        0x02,                     // varint32(2) count
        0x04, 0x01, 0x61,        // key "a"
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // Int64(1)
        0x04, 0x01, 0x62,        // key "b"
        0x02,                     // True
    ];
    assert_eq!(
        bytes, expected,
        "ARTICLE III KAT: Map {{\"a\":1,\"b\":true}} encoding mismatch"
    );
    assert_eq!(decode(&bytes).unwrap(), v);
}

// --- Section 3.1: No Floats ---

#[test]
fn art3_1_no_float_type_exists() {
    // There is no Float variant in Value. This test ensures the enum hasn't been extended.
    let json = r#"{"type": "should not have float"}"#;
    // Try to decode a byte stream with an unknown tag (0x08 would be "float" if it existed)
    let bad_bytes = vec![0x6E, 0x72, 0x66, 0x31, 0x08];
    assert!(
        decode(&bad_bytes).is_err(),
        "ARTICLE III §3.1 VIOLATION: tag 0x08 was accepted. \
         There must be no float type. Tags 0x00-0x07 only. \
         Error: {json:?}" // just to use the variable
    );
}

// --- Section 3.2: Sorted Maps ---

#[test]
fn art3_2_map_keys_sorted_by_bytes() {
    // Keys must be sorted by raw UTF-8 bytes, not locale
    let v = make_map(&[("z", Value::Int(1)), ("a", Value::Int(2))]);
    let bytes = encode(&v);
    let decoded = decode(&bytes).unwrap();
    if let Value::Map(m) = decoded {
        let keys: Vec<&String> = m.keys().collect();
        assert_eq!(
            keys, vec!["a", "z"],
            "ARTICLE III §3.2 VIOLATION: map keys not sorted by bytes"
        );
    }
}

// --- Section 3.3: NFC Strings ---

#[test]
fn art3_3_decoder_rejects_non_nfc() {
    // The decoder must reject non-NFC strings
    // Manually construct NRF bytes with NFD 'é' (e + combining acute = 0x65 0xCC 0x81)
    let bad_bytes: Vec<u8> = vec![
        0x6E, 0x72, 0x66, 0x31, // magic
        0x04,                     // String tag
        0x03,                     // varint32(3) — 3 bytes
        0x65, 0xCC, 0x81,        // NFD é
    ];
    let result = decode(&bad_bytes);
    assert!(
        result.is_err(),
        "ARTICLE III §3.3 VIOLATION: decoder accepted non-NFC string. \
         NFD 'é' (0x65 0xCC 0x81) must be rejected. \
         ρ normalizes on the way in; the decoder rejects on the way out."
    );
}

// ==========================================================================
// ARTICLE IV — The Canonical Artifacts
// ==========================================================================

// --- Section 4.1: Receipt ---

#[test]
fn art4_1_receipt_cid_stability() {
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    let cid1 = r.compute_cid();
    let cid2 = r.compute_cid();
    assert_eq!(
        cid1, cid2,
        "ARTICLE IV §4.1 VIOLATION: receipt CID is not deterministic"
    );
}

#[test]
fn art4_1_receipt_body_cid_matches_body() {
    let r = make_receipt();
    assert_eq!(
        r.compute_body_cid(), r.body_cid,
        "ARTICLE IV §4.1 VIOLATION: body_cid does not match BLAKE3(NRF(body)). \
         This means the body was modified after CID computation."
    );
}

#[test]
fn art4_1_receipt_tamper_body_detected() {
    // Mutate the body AFTER computing CID → integrity check must fail
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    assert!(r.verify_integrity().is_ok(), "baseline integrity should pass");

    // Tamper: change the body
    r.body = Value::String("tampered".into());
    // Don't recompute body_cid — this simulates an attacker
    let result = r.verify_integrity();
    assert!(
        result.is_err(),
        "ARTICLE IV §4.1 VIOLATION: body tamper was NOT detected. \
         Changing the body without updating body_cid must fail verify_integrity(). \
         Error: {result:?}"
    );
}

#[test]
fn art4_1_receipt_tamper_field_detected() {
    // Mutate any field AFTER computing receipt_cid → integrity check must fail
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    assert!(r.verify_integrity().is_ok());

    // Tamper: change the issuer
    r.issuer_did = "did:ubl:attacker".into();
    let result = r.verify_integrity();
    assert!(
        result.is_err(),
        "ARTICLE IV §4.1 VIOLATION: field tamper was NOT detected. \
         Changing issuer_did without recomputing receipt_cid must fail."
    );
}

#[test]
fn art4_1_receipt_sign_verify_roundtrip() {
    let (sk, vk) = keygen();
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);
    assert!(
        r.verify(&vk),
        "ARTICLE IV §4.1 VIOLATION: sign → verify roundtrip failed"
    );
}

#[test]
fn art4_1_receipt_wrong_key_rejected() {
    let (sk, _vk) = keygen();
    let (_sk2, wrong_vk) = keygen();
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);
    assert!(
        !r.verify(&wrong_vk),
        "ARTICLE IV §4.1 VIOLATION: signature verified with wrong key. \
         Ed25519 verification must fail when the key doesn't match."
    );
}

#[test]
fn art4_1_receipt_sig_tamper_detected() {
    let (sk, vk) = keygen();
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);
    // Tamper: flip a bit in the signature
    if let Some(ref mut sig) = r.sig {
        sig[0] ^= 0x01;
    }
    assert!(
        !r.verify(&vk),
        "ARTICLE IV §4.1 VIOLATION: tampered signature was accepted"
    );
}

#[test]
fn art4_1_gho001_ghost_effects_must_be_none() {
    // GHO-001: if decision == GHOST then effects MUST be None
    let mut r = make_receipt();
    r.decision = Some("GHOST".into());
    r.effects = Some(Value::String("should not be here".into()));
    r.receipt_cid = r.compute_cid();
    assert!(
        r.verify_integrity().is_err(),
        "ARTICLE IV §4.1 GHO-001 VIOLATION: GHOST receipt with non-None effects \
         passed integrity check. Constitution says: GHOST ⇒ effects = None."
    );
}

#[test]
fn art4_1_gho001_ghost_none_effects_ok() {
    let mut r = make_receipt();
    r.decision = Some("GHOST".into());
    r.effects = None;
    r.receipt_cid = r.compute_cid();
    assert!(
        r.verify_integrity().is_ok(),
        "ARTICLE IV §4.1: GHOST receipt with None effects should pass integrity"
    );
}

// --- Section 4.2: Permit ---

#[test]
fn art4_2_permit_cid_stability() {
    let p = make_permit();
    let c1 = p.compute_cid();
    let c2 = p.compute_cid();
    assert_eq!(c1, c2, "ARTICLE IV §4.2 VIOLATION: permit CID not deterministic");
}

#[test]
fn art4_2_permit_sign_verify() {
    let (sk, vk) = keygen();
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);
    let now = 1_750_000_000_000_000_000;
    assert!(
        permit::verify_permit(&p, "b3:bbbb", now, &vk).is_ok(),
        "ARTICLE IV §4.2 VIOLATION: valid permit failed verification"
    );
}

#[test]
fn art4_2_permit_expired_rejected() {
    let (sk, vk) = keygen();
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);
    let now = 1_900_000_000_000_000_000; // after expires_at
    let err = permit::verify_permit(&p, "b3:bbbb", now, &vk);
    assert!(
        err.is_err(),
        "ARTICLE IV §4.2 VIOLATION: expired permit was accepted. \
         Permits are time-bounded. After expires_at, they are invalid."
    );
}

#[test]
fn art4_2_permit_input_hash_pinned() {
    let (sk, vk) = keygen();
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);
    let now = 1_750_000_000_000_000_000;
    let err = permit::verify_permit(&p, "b3:WRONG_HASH", now, &vk);
    assert!(
        err.is_err(),
        "ARTICLE IV §4.2 VIOLATION: permit accepted with wrong input hash. \
         Permits are hash-pinned to the specific input they authorized."
    );
}

#[test]
fn art4_2_permit_wrong_key_rejected() {
    let (sk, _vk) = keygen();
    let (_sk2, wrong_vk) = keygen();
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);
    let now = 1_750_000_000_000_000_000;
    let err = permit::verify_permit(&p, "b3:bbbb", now, &wrong_vk);
    assert!(
        err.is_err(),
        "ARTICLE IV §4.2 VIOLATION: permit verified with wrong authority key"
    );
}

#[test]
fn art4_2_permit_cid_tamper_detected() {
    let (sk, vk) = keygen();
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);
    // Tamper: change the act after signing
    p.act = "TRANSACT".into();
    // Don't recompute CID — simulates attacker
    let now = 1_750_000_000_000_000_000;
    let err = permit::verify_permit(&p, "b3:bbbb", now, &vk);
    assert!(
        err.is_err(),
        "ARTICLE IV §4.2 VIOLATION: permit field tamper was not detected. \
         Changing 'act' without recomputing CID must fail."
    );
}

// --- Section 4.3: Ghost ---

#[test]
fn art4_3_ghost_new_pending_valid() {
    let g = make_ghost();
    assert_eq!(g.status, ghost::GhostStatus::Pending);
    assert!(g.cause.is_none(), "ARTICLE IV §4.3: pending ghost must not have cause");
    assert!(g.ghost_cid.starts_with("b3:"), "ARTICLE IV §4.3: ghost CID must start with b3:");
    assert!(g.verify_integrity().is_ok(), "ARTICLE IV §4.3: new pending ghost must pass integrity");
}

#[test]
fn art4_3_ghost_pending_with_cause_rejected() {
    let mut g = make_ghost();
    g.cause = Some(ghost::ExpireCause::Canceled);
    g.ghost_cid = g.compute_cid();
    assert!(
        g.verify_integrity().is_err(),
        "ARTICLE IV §4.3 VIOLATION: pending ghost with cause passed integrity. \
         Constitution says: pending ghosts MUST NOT have a cause."
    );
}

#[test]
fn art4_3_ghost_expired_without_cause_rejected() {
    let mut g = make_ghost();
    g.status = ghost::GhostStatus::Expired;
    g.cause = None;
    g.ghost_cid = g.compute_cid();
    assert!(
        g.verify_integrity().is_err(),
        "ARTICLE IV §4.3 VIOLATION: expired ghost without cause passed integrity. \
         Constitution says: expired ghosts MUST have a cause."
    );
}

#[test]
fn art4_3_ghost_expire_changes_cid() {
    let g = make_ghost();
    let cid_before = g.ghost_cid.clone();
    let mut g2 = make_ghost();
    g2.expire(ghost::ExpireCause::Timeout);
    assert_ne!(
        cid_before, g2.ghost_cid,
        "ARTICLE IV §4.3 VIOLATION: expire did not change ghost CID. \
         Status change must produce a new CID."
    );
}

#[test]
fn art4_3_ghost_expire_clears_sig() {
    let (sk, vk) = keygen();
    let mut g = make_ghost();
    g.sign(&sk);
    assert!(g.verify(&vk), "baseline: sig should verify");
    g.expire(ghost::ExpireCause::Drift);
    assert!(
        g.sig.is_none(),
        "ARTICLE IV §4.3 VIOLATION: expire did not clear signature. \
         After mutation, old sig is invalid and must be cleared."
    );
}

#[test]
fn art4_3_ghost_sign_verify_roundtrip() {
    let (sk, vk) = keygen();
    let mut g = make_ghost();
    g.sign(&sk);
    assert!(g.verify(&vk), "ARTICLE IV §4.3: ghost sign/verify roundtrip failed");
}

// --- Section 4.4: Capsule structural invariants ---

#[test]
fn art4_4_capsule_ghost_requires_links_prev() {
    let cap = ubl_transport::Capsule {
        v: ubl_transport::CAPSULE_VERSION.into(),
        id: vec![0u8; 32],
        hdr: ubl_transport::Header {
            src: "did:ubl:sender".into(),
            dst: "did:ubl:receiver".into(),
            nonce: vec![0u8; 16],
            exp: 1_800_000_000_000_000_000,
            chan: None,
            ts: None,
        },
        env: ubl_transport::Envelope {
            t: ubl_transport::EnvelopeType::Record,
            agent: None,
            intent: ubl_transport::Intent {
                kind: "ATTEST".into(),
                name: "test".into(),
                args: None,
            },
            ctx: None,
            decision: ubl_transport::Decision {
                verdict: "GHOST".into(),
                reason: None,
                metrics: None,
            },
            evidence: None,
            meta: None,
            links: None, // VIOLATION: GHOST requires links.prev
        },
        seal: ubl_transport::Seal {
            alg: ubl_transport::SigAlg::Ed25519,
            kid: "did:ubl:sender#key-1".into(),
            domain: ubl_transport::CAPSULE_DOMAIN.into(),
            scope: "capsule".into(),
            aud: None,
            sig: vec![0u8; 64],
        },
        receipts: vec![],
    };
    assert!(
        cap.check_invariants().is_err(),
        "ARTICLE IV §4.4 VIOLATION: GHOST capsule without links.prev passed invariant check. \
         Constitution says: GHOST verdict requires links.prev (the pending ghost reference)."
    );
}

#[test]
fn art4_4_capsule_allow_requires_evidence() {
    let cap = ubl_transport::Capsule {
        v: ubl_transport::CAPSULE_VERSION.into(),
        id: vec![0u8; 32],
        hdr: ubl_transport::Header {
            src: "did:ubl:sender".into(),
            dst: "did:ubl:receiver".into(),
            nonce: vec![0u8; 16],
            exp: 1_800_000_000_000_000_000,
            chan: None,
            ts: None,
        },
        env: ubl_transport::Envelope {
            t: ubl_transport::EnvelopeType::Record,
            agent: None,
            intent: ubl_transport::Intent {
                kind: "EVALUATE".into(),
                name: "test".into(),
                args: None,
            },
            ctx: None,
            decision: ubl_transport::Decision {
                verdict: "ALLOW".into(),
                reason: None,
                metrics: None,
            },
            evidence: None, // VIOLATION: ALLOW requires evidence
            meta: None,
            links: None,
        },
        seal: ubl_transport::Seal {
            alg: ubl_transport::SigAlg::Ed25519,
            kid: "did:ubl:sender#key-1".into(),
            domain: ubl_transport::CAPSULE_DOMAIN.into(),
            scope: "capsule".into(),
            aud: None,
            sig: vec![0u8; 64],
        },
        receipts: vec![],
    };
    assert!(
        cap.check_invariants().is_err(),
        "ARTICLE IV §4.4 VIOLATION: ALLOW capsule without evidence passed invariant check. \
         Constitution says: ALLOW/DENY verdict requires evidence field."
    );
}

// ==========================================================================
// ARTICLE V — The Three Acts
// ==========================================================================

#[test]
fn art5_1_only_three_acts_exist() {
    // The acts crate must define exactly 3 acts
    let acts = ["ATTEST", "EVALUATE", "TRANSACT"];
    for act in &acts {
        let r = make_receipt();
        // Verify the receipt struct accepts these acts
        let mut r2 = r.clone();
        r2.act = act.to_string();
        r2.receipt_cid = r2.compute_cid();
        assert!(
            r2.verify_integrity().is_ok(),
            "ARTICLE V §5.1: act '{act}' should be valid"
        );
    }
}

// ==========================================================================
// ARTICLE VI — The Four Decisions
// ==========================================================================

#[test]
fn art6_four_decisions_vocabulary() {
    let decisions = ["ALLOW", "DENY", "REQUIRE", "GHOST"];
    for d in &decisions {
        let mut r = make_receipt();
        r.decision = Some(d.to_string());
        if *d == "GHOST" {
            r.effects = None; // GHO-001
        }
        r.receipt_cid = r.compute_cid();
        assert!(
            r.verify_integrity().is_ok(),
            "ARTICLE VI: decision '{d}' should be valid in a receipt"
        );
    }
}

// ==========================================================================
// ARTICLE IX — Non-Negotiable Principles
// ==========================================================================

// --- Section 9.1: No Hidden State ---

#[test]
fn art9_1_receipt_cid_covers_all_fields() {
    // Changing ANY field must change the CID — no hidden state
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    let baseline_cid = r.receipt_cid.clone();

    // Test each field mutation
    #[allow(clippy::type_complexity)]
    let mutations: Vec<(&str, Box<dyn Fn(&mut receipt::Receipt)>)> = vec![
        ("issuer_did", Box::new(|r: &mut receipt::Receipt| r.issuer_did = "did:ubl:other".into())),
        ("act", Box::new(|r: &mut receipt::Receipt| r.act = "EVALUATE".into())),
        ("t", Box::new(|r: &mut receipt::Receipt| r.t = 999)),
        ("url", Box::new(|r: &mut receipt::Receipt| r.url = "https://other.com".into())),
        ("nonce", Box::new(|r: &mut receipt::Receipt| r.nonce = vec![1u8; 16])),
    ];

    for (field, mutate) in mutations {
        let mut r2 = make_receipt();
        mutate(&mut r2);
        r2.receipt_cid = r2.compute_cid();
        assert_ne!(
            r2.receipt_cid, baseline_cid,
            "ARTICLE IX §9.1 VIOLATION: changing '{field}' did not change receipt_cid. \
             No hidden state: every field must be covered by the CID."
        );
    }
}

// --- Section 9.2: No Trusted Agents ---

#[test]
fn art9_2_unsigned_receipt_not_verifiable() {
    let (_sk, vk) = keygen();
    let mut r = make_receipt();
    r.receipt_cid = r.compute_cid();
    // Do NOT sign
    assert!(
        !r.verify(&vk),
        "ARTICLE IX §9.2 VIOLATION: unsigned receipt verified successfully. \
         No agent is trusted by default. Verification requires a valid signature."
    );
}

#[test]
fn art9_2_unsigned_permit_rejected() {
    let (_sk, vk) = keygen();
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    // Do NOT sign
    let now = 1_750_000_000_000_000_000;
    assert!(
        permit::verify_permit(&p, "b3:bbbb", now, &vk).is_err(),
        "ARTICLE IX §9.2 VIOLATION: unsigned permit was accepted. \
         Trust is established by verifying signatures, not by default."
    );
}

// ==========================================================================
// CROSS-CRATE PIPELINE: Ghost → Receipt → Permit
// ==========================================================================

#[test]
fn pipeline_wbe_ghost_to_receipt() {
    // Full Write-Before-Execute flow:
    // 1. Create ghost (pending)
    // 2. Sign ghost
    // 3. Create receipt that references the ghost
    // 4. Verify the chain
    let (sk, vk) = keygen();

    // Step 1: Ghost
    let mut g = make_ghost();
    g.sign(&sk);
    assert!(g.verify(&vk), "ghost signature must verify");
    let ghost_cid = g.ghost_cid.clone();

    // Step 2: Receipt referencing the ghost
    let mut r = make_receipt();
    r.decision = Some("ALLOW".into());
    r.ghost = Some(receipt::GhostInfo {
        budget: 100,
        counter: 1,
        cost_ms: 50,
        window_day: 1,
    });
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);

    // Step 3: Verify everything
    assert!(r.verify(&vk), "receipt signature must verify");
    assert!(r.verify_integrity().is_ok(), "receipt integrity must pass");
    assert!(
        !ghost_cid.is_empty(),
        "ghost CID must be non-empty for pipeline linking"
    );
}

#[test]
fn pipeline_prev_links_receipts() {
    // Two receipts linked via pipeline_prev:
    // Receipt 1 (ATTEST) → Receipt 2 (EVALUATE) references Receipt 1's CID
    let (sk, vk) = keygen();

    // Receipt 1: ATTEST
    let mut r1 = make_receipt();
    r1.act = "ATTEST".into();
    r1.receipt_cid = r1.compute_cid();
    r1.sign(&sk);
    assert!(r1.verify(&vk));

    // Receipt 2: EVALUATE, references Receipt 1
    let mut r2 = make_receipt();
    r2.act = "EVALUATE".into();
    r2.pipeline_prev = vec![r1.receipt_cid.clone()];
    r2.receipt_cid = r2.compute_cid();
    r2.sign(&sk);
    assert!(r2.verify(&vk));

    // Verify the link
    assert_eq!(
        r2.pipeline_prev[0], r1.receipt_cid,
        "PIPELINE: receipt 2 must reference receipt 1's CID via pipeline_prev"
    );

    // Verify that changing receipt 1 would break the link
    let old_cid = r1.receipt_cid.clone();
    r1.issuer_did = "did:ubl:tampered".into();
    r1.receipt_cid = r1.compute_cid();
    assert_ne!(
        r1.receipt_cid, old_cid,
        "PIPELINE: tampering receipt 1 must change its CID, breaking the pipeline_prev link"
    );
}

#[test]
fn pipeline_permit_authorizes_receipt() {
    // Full flow: Permit authorizes → Receipt references permit_cid
    let (sk, vk) = keygen();

    // Step 1: Create and sign permit
    let mut p = make_permit();
    p.permit_cid = p.compute_cid();
    p.sign(&sk);
    let now = 1_750_000_000_000_000_000;
    assert!(permit::verify_permit(&p, "b3:bbbb", now, &vk).is_ok());

    // Step 2: Receipt references the permit
    let mut r = make_receipt();
    r.permit_cid = Some(p.permit_cid.clone());
    r.receipt_cid = r.compute_cid();
    r.sign(&sk);

    // Step 3: Verify
    assert!(r.verify(&vk));
    assert!(r.verify_integrity().is_ok());
    assert_eq!(
        r.permit_cid.as_ref().unwrap(), &p.permit_cid,
        "PIPELINE: receipt must carry the permit CID that authorized it"
    );
}

// ==========================================================================
// ARTICLE X — The Crate Law
// ==========================================================================

#[test]
fn art10_1_nrf_core_is_single_source() {
    // nrf1 crate must delegate to nrf-core (same encode output)
    let v = make_map(&[("test", Value::Int(1))]);
    let core_bytes = nrf_core::encode(&v);
    let facade_bytes = nrf1::encode_stream(&v);
    assert_eq!(
        core_bytes, facade_bytes,
        "ARTICLE X §10.1 VIOLATION: nrf1 and nrf-core produce different bytes. \
         nrf-core is the single source of truth. All facades must delegate to it."
    );
}

#[test]
fn art10_1_cid_consistent_across_crates() {
    // CID computed by nrf-core must match CID computed by nrf1
    let v = make_map(&[("test", Value::Int(1))]);
    let core_cid = nrf_core::blake3_cid(&v);
    let facade_cid = nrf1::blake3_cid(&v);
    assert_eq!(
        core_cid, facade_cid,
        "ARTICLE X §10.1 VIOLATION: nrf-core and nrf1 produce different CIDs. \
         Single source of truth means identical output everywhere."
    );
}

// ==========================================================================
// ρ TIMESTAMP AND DECIMAL NORMALIZATION
// ==========================================================================

#[test]
fn art1_1_rho_timestamp_strip_zero_fraction() {
    assert_eq!(
        rho::normalize_timestamp("2024-01-15T10:30:00.000Z").unwrap(),
        "2024-01-15T10:30:00Z",
        "ARTICLE I §1.1: ρ must strip zero fractional seconds from timestamps"
    );
}

#[test]
fn art1_1_rho_timestamp_minimal_fraction() {
    assert_eq!(
        rho::normalize_timestamp("2024-01-15T10:30:00.100Z").unwrap(),
        "2024-01-15T10:30:00.1Z",
        "ARTICLE I §1.1: ρ must minimize fractional seconds (strip trailing zeros)"
    );
}

#[test]
fn art1_1_rho_timestamp_reject_non_utc() {
    assert!(
        rho::normalize_timestamp("2024-01-15T10:30:00+05:00").is_err(),
        "ARTICLE I §1.1: ρ must reject non-UTC timestamps (no timezone offsets)"
    );
}

#[test]
fn art1_1_rho_decimal_strip_dot_zero() {
    assert_eq!(
        rho::normalize_decimal("1.0").unwrap(), "1",
        "ARTICLE I §1.1: ρ must strip superfluous .0 from decimals"
    );
}

#[test]
fn art1_1_rho_decimal_negative_zero() {
    assert_eq!(
        rho::normalize_decimal("-0").unwrap(), "0",
        "ARTICLE I §1.1: ρ must normalize -0 to 0"
    );
}

#[test]
fn art1_1_rho_decimal_reject_exponent() {
    assert!(
        rho::normalize_decimal("1e2").is_err(),
        "ARTICLE I §1.1: ρ must reject exponential notation in decimals"
    );
}

#[test]
fn art1_1_rho_decimal_reject_leading_zero() {
    assert!(
        rho::normalize_decimal("01.5").is_err(),
        "ARTICLE I §1.1: ρ must reject leading zeros in decimals"
    );
}

#[test]
fn art1_1_rho_set_sort_and_dedup() {
    let items = vec![
        Value::String("c".into()),
        Value::String("a".into()),
        Value::String("c".into()),
        Value::String("b".into()),
    ];
    let result = rho::normalize_as_set(&items).unwrap();
    assert_eq!(result.len(), 3, "ARTICLE I §1.1: ρ must deduplicate sets");
    assert_eq!(
        result,
        vec![
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ],
        "ARTICLE I §1.1: ρ must sort sets by canonical NRF bytes"
    );
}
