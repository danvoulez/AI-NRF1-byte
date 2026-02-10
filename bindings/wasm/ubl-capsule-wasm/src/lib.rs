use js_sys::Uint8Array;
use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

fn bytes_to_capsule(capsule_bytes: &[u8]) -> Result<ubl_capsule::Capsule, JsError> {
    let v =
        nrf_core::decode(capsule_bytes).map_err(|e| JsError::new(&format!("InvalidNrf: {e}")))?;
    let json = nrf_value_to_json(&v);
    serde_json::from_value(json).map_err(|e| JsError::new(&format!("InvalidCapsule: {e}")))
}

fn nrf_value_to_json(v: &nrf_core::Value) -> serde_json::Value {
    match v {
        nrf_core::Value::Null => serde_json::Value::Null,
        nrf_core::Value::Bool(b) => serde_json::Value::Bool(*b),
        nrf_core::Value::Int(i) => serde_json::Value::Number((*i).into()),
        nrf_core::Value::String(s) => serde_json::Value::String(s.clone()),
        nrf_core::Value::Bytes(b) => serde_json::Value::String(hex::encode(b)),
        nrf_core::Value::Array(a) => {
            serde_json::Value::Array(a.iter().map(nrf_value_to_json).collect())
        }
        nrf_core::Value::Map(m) => {
            let mut o = serde_json::Map::new();
            for (k, v) in m {
                o.insert(k.clone(), nrf_value_to_json(v));
            }
            serde_json::Value::Object(o)
        }
    }
}

/// Verify a capsule's seal (domain/scope/aud/expiry/id + Ed25519 signature).
///
/// Inputs:
/// - `capsule`: JS object compatible with `ubl_capsule::Capsule` serde shape (JSON-style fields).
/// - `pkBytes`: Uint8Array(32) Ed25519 public key.
/// - `allowedSkewNs`: clock skew tolerance, nanoseconds.
#[wasm_bindgen(js_name = "verifySeal")]
pub fn js_verify_seal(
    capsule: JsValue,
    pk_bytes: Uint8Array,
    allowed_skew_ns: i64,
) -> Result<(), JsError> {
    let capsule: ubl_capsule::Capsule = serde_wasm_bindgen::from_value(capsule)
        .map_err(|e| JsError::new(&format!("InvalidCapsule: {e}")))?;

    let pk_vec = pk_bytes.to_vec();
    let pk_arr: [u8; 32] = pk_vec
        .try_into()
        .map_err(|_| JsError::new("InvalidPublicKey: expected 32 bytes"))?;
    let pk = ed25519_dalek::VerifyingKey::from_bytes(&pk_arr)
        .map_err(|_| JsError::new("InvalidPublicKey: decode failed"))?;

    let opts = ubl_capsule::seal::VerifyOpts { allowed_skew_ns };
    ubl_capsule::seal::verify_with_opts(&capsule, &pk, &opts)
        .map_err(|e| JsError::new(&e.to_string()))
}

/// Verify a capsule's seal from canonical capsule bytes (ai-nrf1 stream).
///
/// Inputs:
/// - `capsuleBytes`: Uint8Array of ai-nrf1 bytes encoding the capsule map.
/// - `pkBytes`: Uint8Array(32) Ed25519 public key.
/// - `allowedSkewNs`: clock skew tolerance, nanoseconds.
#[wasm_bindgen(js_name = "verifySealBytes")]
pub fn js_verify_seal_bytes(
    capsule_bytes: &[u8],
    pk_bytes: Uint8Array,
    allowed_skew_ns: i64,
) -> Result<(), JsError> {
    let capsule = bytes_to_capsule(capsule_bytes)?;
    js_verify_seal(
        serde_wasm_bindgen::to_value(&capsule)?,
        pk_bytes,
        allowed_skew_ns,
    )
}

#[derive(Serialize)]
struct ChainReport {
    ok: bool,
    hops: usize,
}

/// Verify the receipts chain for a capsule.
///
/// Inputs:
/// - `capsule`: JS object compatible with `ubl_capsule::Capsule`.
/// - `keyringHex`: JS object mapping `node_did#key` -> hex-encoded Ed25519 public key (32 bytes).
///
/// Returns: `{ ok: true, hops: <n> }` on success.
#[wasm_bindgen(js_name = "verifyReceiptsChain")]
pub fn js_verify_receipts_chain(
    capsule: JsValue,
    keyring_hex: JsValue,
) -> Result<JsValue, JsError> {
    let capsule: ubl_capsule::Capsule = serde_wasm_bindgen::from_value(capsule)
        .map_err(|e| JsError::new(&format!("InvalidCapsule: {e}")))?;

    let keyring: HashMap<String, String> = serde_wasm_bindgen::from_value(keyring_hex)
        .map_err(|e| JsError::new(&format!("InvalidKeyring: {e}")))?;

    let mut pks: HashMap<String, ed25519_dalek::VerifyingKey> = HashMap::new();
    for (node, hex_pk) in keyring {
        let bytes =
            hex::decode(hex_pk.trim()).map_err(|_| JsError::new("InvalidKeyring: bad hex"))?;
        let pk_arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| JsError::new("InvalidKeyring: pk must be 32 bytes"))?;
        let pk = ed25519_dalek::VerifyingKey::from_bytes(&pk_arr)
            .map_err(|_| JsError::new("InvalidKeyring: pk decode failed"))?;
        pks.insert(node, pk);
    }

    let resolve = |node: &str| -> Option<ed25519_dalek::VerifyingKey> { pks.get(node).copied() };
    ubl_capsule::receipt::verify_chain(&capsule.id, &capsule.receipts, &resolve)
        .map_err(|e| JsError::new(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&ChainReport {
        ok: true,
        hops: capsule.receipts.len(),
    })
    .map_err(|e| JsError::new(&format!("SerializeError: {e}")))
}

/// Verify the receipts chain from canonical capsule bytes (ai-nrf1 stream).
#[wasm_bindgen(js_name = "verifyReceiptsChainBytes")]
pub fn js_verify_receipts_chain_bytes(
    capsule_bytes: &[u8],
    keyring_hex: JsValue,
) -> Result<JsValue, JsError> {
    let capsule = bytes_to_capsule(capsule_bytes)?;
    js_verify_receipts_chain(serde_wasm_bindgen::to_value(&capsule)?, keyring_hex)
}

#[wasm_bindgen(js_name = "version")]
pub fn js_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
