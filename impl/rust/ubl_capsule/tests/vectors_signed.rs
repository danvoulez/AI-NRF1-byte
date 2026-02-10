use std::path::{Path, PathBuf};

use ubl_capsule::types::Capsule;

fn repo_root() -> PathBuf {
    // impl/rust/ubl_capsule
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.join("../../../")
}

fn read_pk(path: &Path) -> ed25519_dalek::VerifyingKey {
    let hex_str = std::fs::read_to_string(path).expect("read pk");
    let bytes = hex::decode(hex_str.trim()).expect("pk hex");
    let arr: [u8; 32] = bytes.try_into().expect("pk len");
    ed25519_dalek::VerifyingKey::from_bytes(&arr).expect("pk bytes")
}

fn value_to_json(v: &nrf_core::Value) -> serde_json::Value {
    match v {
        nrf_core::Value::Null => serde_json::Value::Null,
        nrf_core::Value::Bool(b) => serde_json::Value::Bool(*b),
        nrf_core::Value::Int(i) => serde_json::Value::Number((*i).into()),
        nrf_core::Value::String(s) => serde_json::Value::String(s.clone()),
        nrf_core::Value::Bytes(b) => serde_json::Value::String(hex::encode(b)),
        nrf_core::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(value_to_json).collect())
        }
        nrf_core::Value::Map(m) => {
            let mut o = serde_json::Map::new();
            for (k, v) in m {
                o.insert(k.clone(), value_to_json(v));
            }
            serde_json::Value::Object(o)
        }
    }
}

fn capsule_from_signed_nrf(bytes: &[u8]) -> Capsule {
    let v = nrf_core::decode(bytes).expect("decode nrf");
    // Check canonical re-encode
    assert_eq!(nrf_core::encode(&v), bytes, "vector must be canonical");

    let json = value_to_json(&v);
    serde_json::from_value(json).expect("value -> capsule json -> capsule")
}

#[test]
fn verify_signed_capsule_vectors() {
    let root = repo_root();
    let base = root.join("tests/vectors/capsule");

    let pk = read_pk(&base.join("alice.pk"));

    for name in ["capsule_ack", "capsule_ask", "capsule_nack"] {
        let signed_nrf = base.join(format!("{name}.signed.nrf"));
        assert!(
            signed_nrf.exists(),
            "missing vector file: {}",
            signed_nrf.display()
        );

        let bytes = std::fs::read(&signed_nrf).expect("read vector");
        let capsule = capsule_from_signed_nrf(&bytes);

        assert_ne!(capsule.id, [0u8; 32], "signed vector must have non-zero id");
        assert_ne!(
            capsule.seal.sig, [0u8; 64],
            "signed vector must have non-zero sig"
        );

        ubl_capsule::seal::verify(&capsule, &pk).expect("seal verify");
    }
}

#[test]
fn verify_chain2_vector() {
    let root = repo_root();
    let base = root.join("tests/vectors/capsule");

    let pk = read_pk(&base.join("alice.pk"));
    let signed_nrf = base.join("capsule_ack.chain2.signed.nrf");
    assert!(
        signed_nrf.exists(),
        "missing vector file: {}",
        signed_nrf.display()
    );

    let bytes = std::fs::read(&signed_nrf).expect("read vector");
    let capsule = capsule_from_signed_nrf(&bytes);
    ubl_capsule::seal::verify(&capsule, &pk).expect("seal verify");

    let keyring_path = base.join("keyring.json");
    let keyring_s = std::fs::read_to_string(&keyring_path).expect("read keyring");
    let keyring: std::collections::HashMap<String, String> =
        serde_json::from_str(&keyring_s).expect("parse keyring");
    let mut pks: std::collections::HashMap<String, ed25519_dalek::VerifyingKey> =
        std::collections::HashMap::new();
    for (node, pk_hex) in keyring {
        let bytes = hex::decode(pk_hex.trim()).expect("pk hex");
        let arr: [u8; 32] = bytes.try_into().expect("pk len");
        let pk = ed25519_dalek::VerifyingKey::from_bytes(&arr).expect("pk bytes");
        pks.insert(node, pk);
    }
    let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> { pks.get(node).copied() };
    ubl_capsule::receipt::verify_chain(&capsule.id, &capsule.receipts, &resolve)
        .expect("verify chain");
    assert_eq!(capsule.receipts.len(), 2);
}

#[test]
fn expired_and_tampered_vectors_fail() {
    let root = repo_root();
    let base = root.join("tests/vectors/capsule");

    let pk = read_pk(&base.join("alice.pk"));

    let expired_nrf = base.join("capsule_expired.signed.nrf");
    assert!(
        expired_nrf.exists(),
        "missing vector file: {}",
        expired_nrf.display()
    );
    let bytes = std::fs::read(&expired_nrf).expect("read vector");
    let capsule = capsule_from_signed_nrf(&bytes);
    let err = ubl_capsule::seal::verify(&capsule, &pk).unwrap_err();
    assert!(matches!(err, ubl_capsule::seal::SealError::Expired { .. }));

    let tampered_nrf = base.join("capsule_ack.tampered.signed.nrf");
    assert!(
        tampered_nrf.exists(),
        "missing vector file: {}",
        tampered_nrf.display()
    );
    let bytes = std::fs::read(&tampered_nrf).expect("read vector");
    let capsule = capsule_from_signed_nrf(&bytes);
    let err = ubl_capsule::seal::verify(&capsule, &pk).unwrap_err();
    assert_eq!(err, ubl_capsule::seal::SealError::IdMismatch);
}
