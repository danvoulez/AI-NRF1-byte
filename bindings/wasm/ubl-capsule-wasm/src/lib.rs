use js_sys::Uint8Array;
use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

fn js_now_nanos_i64() -> i64 {
    // Date.now() returns epoch-millis as f64.
    let ms = js_sys::Date::now();
    let ns = ms * 1_000_000.0;
    if !ns.is_finite() || ns <= 0.0 {
        return i64::MAX;
    }
    // Clamp to i64.
    if ns >= i64::MAX as f64 {
        i64::MAX
    } else {
        ns as i64
    }
}

fn pk_from_uint8array(pk_bytes: Uint8Array) -> Result<ed25519_dalek::VerifyingKey, JsError> {
    let pk_vec = pk_bytes.to_vec();
    let pk_arr: [u8; 32] = pk_vec
        .try_into()
        .map_err(|_| JsError::new("InvalidPublicKey: expected 32 bytes"))?;
    ed25519_dalek::VerifyingKey::from_bytes(&pk_arr)
        .map_err(|_| JsError::new("InvalidPublicKey: decode failed"))
}

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

    let pk = pk_from_uint8array(pk_bytes)?;

    let opts = ubl_capsule::seal::VerifyOpts {
        allowed_skew_ns,
        now_ns: Some(js_now_nanos_i64()),
    };
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
    let pk = pk_from_uint8array(pk_bytes)?;
    let opts = ubl_capsule::seal::VerifyOpts {
        allowed_skew_ns,
        now_ns: Some(js_now_nanos_i64()),
    };
    ubl_capsule::seal::verify_with_opts(&capsule, &pk, &opts)
        .map_err(|e| JsError::new(&e.to_string()))
}

#[derive(Serialize)]
struct ChainReport {
    ok: bool,
    hops: usize,
}

fn verify_receipts_chain_capsule(
    capsule: &ubl_capsule::Capsule,
    keyring_hex: HashMap<String, String>,
) -> Result<ChainReport, JsError> {
    let mut pks: HashMap<String, ed25519_dalek::VerifyingKey> = HashMap::new();
    for (node, hex_pk) in keyring_hex {
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

    Ok(ChainReport {
        ok: true,
        hops: capsule.receipts.len(),
    })
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

    serde_wasm_bindgen::to_value(&verify_receipts_chain_capsule(&capsule, keyring)?)
        .map_err(|e| JsError::new(&format!("SerializeError: {e}")))
}

/// Verify the receipts chain from canonical capsule bytes (ai-nrf1 stream).
#[wasm_bindgen(js_name = "verifyReceiptsChainBytes")]
pub fn js_verify_receipts_chain_bytes(
    capsule_bytes: &[u8],
    keyring_hex: JsValue,
) -> Result<JsValue, JsError> {
    let capsule = bytes_to_capsule(capsule_bytes)?;
    let keyring: HashMap<String, String> = serde_wasm_bindgen::from_value(keyring_hex)
        .map_err(|e| JsError::new(&format!("InvalidKeyring: {e}")))?;
    serde_wasm_bindgen::to_value(&verify_receipts_chain_capsule(&capsule, keyring)?)
        .map_err(|e| JsError::new(&format!("SerializeError: {e}")))
}

#[wasm_bindgen(js_name = "version")]
pub fn js_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
