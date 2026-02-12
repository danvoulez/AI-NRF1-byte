use nrf_core::Value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// ai-nrf1 WASM bindings
//
// Exposes the canonical encoder/decoder/hash to JavaScript/TypeScript.
// Canon lives in Rust (nrf-core). This is the ONLY way to touch canon
// from JS — never reimplement encode/decode/hash in JS.
// ---------------------------------------------------------------------------

/// Encode a JSON value (as JS object) into canonical ai-nrf1 bytes.
/// Input: any JSON-compatible JS value.
/// Output: Uint8Array of canonical NRF bytes (with "nrf1" magic prefix).
#[wasm_bindgen(js_name = "encode")]
pub fn js_encode(val: JsValue) -> Result<Vec<u8>, JsError> {
    let json: serde_json::Value = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsError::new(&format!("InvalidInput: {e}")))?;
    let nrf_val = json_to_value(&json).map_err(|e| JsError::new(&e))?;
    let normalized =
        nrf_core::rho::normalize(&nrf_val).map_err(|e| JsError::new(&format!("{e}")))?;
    Ok(nrf_core::encode(&normalized))
}

/// Decode canonical ai-nrf1 bytes into a JSON-compatible JS value.
/// Input: Uint8Array of NRF bytes.
/// Output: JS object (JSON-compatible).
#[wasm_bindgen(js_name = "decode")]
pub fn js_decode(bytes: &[u8]) -> Result<JsValue, JsError> {
    let val = nrf_core::decode(bytes).map_err(|e| JsError::new(&format!("{e}")))?;
    let json = value_to_json(&val);
    serde_wasm_bindgen::to_value(&json).map_err(|e| JsError::new(&format!("SerializeError: {e}")))
}

/// Hash raw bytes with BLAKE3. Returns 32-byte Uint8Array.
#[wasm_bindgen(js_name = "hashBytes")]
pub fn js_hash_bytes(data: &[u8]) -> Vec<u8> {
    nrf_core::hash_bytes(data).to_vec()
}

/// Encode a JSON value to canonical NRF bytes, then BLAKE3 hash.
/// Returns 32-byte Uint8Array.
#[wasm_bindgen(js_name = "hashValue")]
pub fn js_hash_value(val: JsValue) -> Result<Vec<u8>, JsError> {
    let bytes = js_encode(val)?;
    Ok(nrf_core::hash_bytes(&bytes).to_vec())
}

/// Compute the canonical CID string: "b3:<hex>" from a JSON value.
/// Applies ρ normalization before encoding.
#[wasm_bindgen(js_name = "canonicalCid")]
pub fn js_canonical_cid(val: JsValue) -> Result<String, JsError> {
    let json: serde_json::Value = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsError::new(&format!("InvalidInput: {e}")))?;
    let nrf_val = json_to_value(&json).map_err(|e| JsError::new(&e))?;
    nrf_core::rho::canonical_cid(&nrf_val).map_err(|e| JsError::new(&format!("{e}")))
}

/// Verify that NRF bytes decode successfully and are canonical.
/// Returns true if valid, throws on error.
#[wasm_bindgen(js_name = "verify")]
pub fn js_verify(bytes: &[u8]) -> Result<bool, JsError> {
    let val = nrf_core::decode(bytes).map_err(|e| JsError::new(&format!("{e}")))?;
    // Re-encode and check roundtrip
    let re_encoded = nrf_core::encode(&val);
    if re_encoded != bytes {
        return Err(JsError::new("NotCanonical: re-encoding differs from input"));
    }
    Ok(true)
}

/// Normalize a JSON value via ρ without encoding.
/// Returns the ρ-normalized JSON value.
#[wasm_bindgen(js_name = "normalize")]
pub fn js_normalize(val: JsValue) -> Result<JsValue, JsError> {
    let json: serde_json::Value = serde_wasm_bindgen::from_value(val)
        .map_err(|e| JsError::new(&format!("InvalidInput: {e}")))?;
    let nrf_val = json_to_value(&json).map_err(|e| JsError::new(&e))?;
    let normalized =
        nrf_core::rho::normalize(&nrf_val).map_err(|e| JsError::new(&format!("{e}")))?;
    let out_json = value_to_json(&normalized);
    serde_wasm_bindgen::to_value(&out_json)
        .map_err(|e| JsError::new(&format!("SerializeError: {e}")))
}

/// Encode hex bytes as lowercase hex string.
#[wasm_bindgen(js_name = "encodeHex")]
pub fn js_encode_hex(bytes: &[u8]) -> String {
    nrf_core::encode_hex_lower(bytes)
}

/// Parse a lowercase hex string into bytes.
#[wasm_bindgen(js_name = "parseHex")]
pub fn js_parse_hex(hex: &str) -> Result<Vec<u8>, JsError> {
    nrf_core::parse_hex_lower(hex).map_err(|e| JsError::new(&format!("{e}")))
}

/// Return the version of the WASM bindings.
#[wasm_bindgen(js_name = "version")]
pub fn js_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// JSON ↔ nrf_core::Value conversion (internal)
// ---------------------------------------------------------------------------

fn json_to_value(j: &serde_json::Value) -> Result<Value, String> {
    match j {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if n.is_f64() {
                Err("Float: ai-nrf1 forbids floating point numbers. Use decimal strings.".into())
            } else {
                Err("InvalidNumber: cannot represent as Int64".into())
            }
        }
        // Canon 4: strings are just strings. No prefix interpretation.
        serde_json::Value::String(s) => Ok(Value::String(s.clone())),
        serde_json::Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(json_to_value(item)?);
            }
            Ok(Value::Array(out))
        }
        serde_json::Value::Object(obj) => {
            // Canon 4: {"$bytes": "<lowercase hex>"} is the ONLY bytes form
            if obj.len() == 1 {
                if let Some(serde_json::Value::String(hex_str)) = obj.get("$bytes") {
                    let bytes = nrf_core::parse_hex_lower(hex_str)
                        .map_err(|e| format!("Err.Canon.BadBytesHex: {e}"))?;
                    return Ok(Value::Bytes(bytes));
                }
            }
            let mut map = BTreeMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v)?);
            }
            Ok(Value::Map(map))
        }
    }
}

fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::json!(*n),
        Value::String(s) => serde_json::Value::String(s.clone()),
        // Canon 4: ALL bytes → {"$bytes": "<lowercase hex>"}
        Value::Bytes(b) => {
            let mut obj = serde_json::Map::new();
            obj.insert("$bytes".into(), serde_json::Value::String(nrf_core::encode_hex_lower(b)));
            serde_json::Value::Object(obj)
        }
        Value::Array(items) => serde_json::Value::Array(items.iter().map(value_to_json).collect()),
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> = m
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests (wasm-bindgen-test)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_value_null() {
        let j = serde_json::Value::Null;
        assert_eq!(json_to_value(&j).unwrap(), Value::Null);
    }

    #[test]
    fn test_json_to_value_int() {
        let j = serde_json::json!(42);
        assert_eq!(json_to_value(&j).unwrap(), Value::Int(42));
    }

    #[test]
    fn test_json_to_value_float_rejected() {
        let j = serde_json::json!(3.14);
        let result = json_to_value(&j);
        // JsError can't be constructed in non-wasm tests, so we check
        // that the function does not return Ok.
        assert!(result.is_err(), "floats must be rejected");
    }

    #[test]
    fn test_json_to_value_bytes_object() {
        // Canon 4: {"$bytes": "<hex>"} is the ONLY bytes form
        let j = serde_json::json!({"$bytes": "deadbeef"});
        let v = json_to_value(&j).unwrap();
        assert_eq!(v, Value::Bytes(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    }

    #[test]
    fn test_b3_prefix_is_just_a_string() {
        // Canon 4: "b3:..." is a plain string, NOT bytes
        let j = serde_json::json!("b3:deadbeef");
        let v = json_to_value(&j).unwrap();
        assert_eq!(v, Value::String("b3:deadbeef".into()));
    }

    #[test]
    fn test_roundtrip_value() {
        let j = serde_json::json!({"a": 1, "b": true, "c": "hello"});
        let v = json_to_value(&j).unwrap();
        let back = value_to_json(&v);
        assert_eq!(j, back);
    }

    #[test]
    fn test_roundtrip_bytes() {
        // Canon 4: bytes roundtrip through {"$bytes": "<hex>"}
        let j = serde_json::json!({"$bytes": "cafebabe"});
        let v = json_to_value(&j).unwrap();
        assert_eq!(v, Value::Bytes(vec![0xCA, 0xFE, 0xBA, 0xBE]));
        let back = value_to_json(&v);
        assert_eq!(back, j);
    }
}
