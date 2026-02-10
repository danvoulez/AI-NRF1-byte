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
