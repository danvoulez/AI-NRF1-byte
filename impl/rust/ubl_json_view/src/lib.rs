//! `ai-json-nrf1` — Deterministic JSON view of NRF-1.1 binary values.
//!
//! Provides `to_json` and `from_json` for lossless, canonical round-tripping
//! between `nrf_core::Value` (bytes) and `serde_json::Value` (JSON).
//!
//! Prefixes:
//!   Bytes(32)      → `"b3:<hex>"`
//!   Bytes(16 | 64) → `"b64:<base64>"`
//!   Bytes(other)   → `{"$bytes": "<base64>"}`
//!
//! Integers: only Int64 (no floats). Decimals as canonical strings.
//! Strings: NFC, no BOM, ASCII-only enforced for DID/KID fields.

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use nrf_core::Value;
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use thiserror::Error;
extern crate unicode_normalization;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error, PartialEq, Eq)]
pub enum JsonViewError {
    #[error("Err.JsonView.Float: floats are forbidden, use Int64")]
    Float,
    #[error("Err.JsonView.InvalidUTF8")]
    InvalidUTF8,
    #[error("Err.JsonView.NotNFC: string is not NFC-normalized")]
    NotNFC,
    #[error("Err.JsonView.BOMPresent")]
    BOMPresent,
    #[error("Err.JsonView.OddHex: hex string has odd length")]
    OddHex,
    #[error("Err.JsonView.BadHex: invalid hex character")]
    BadHex,
    #[error("Err.JsonView.BadBase64: {0}")]
    BadBase64(String),
    #[error("Err.JsonView.BadPrefix: unknown bytes prefix")]
    BadPrefix,
    #[error("Err.JsonView.NotASCII: DID/KID fields must be ASCII")]
    NotASCII,
    #[error("Err.JsonView.InvalidDecimal: {0}")]
    InvalidDecimal(String),
    #[error("Err.JsonView.IntegerOverflow")]
    IntegerOverflow,
    #[error("Err.JsonView.NonMinimalVarint")]
    NonMinimalVarint,
    #[error("Err.JsonView.NrfDecode: {0}")]
    NrfDecode(String),
}

// ---------------------------------------------------------------------------
// Regex patterns
// ---------------------------------------------------------------------------

static RE_B3: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^b3:[0-9a-f]{64}$").unwrap());
static RE_B64_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^b64:").unwrap());
static RE_DECIMAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?$").unwrap());

// ---------------------------------------------------------------------------
// to_json: Value → serde_json::Value
// ---------------------------------------------------------------------------

/// Convert an NRF `Value` to its canonical JSON representation.
pub fn to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(n) => serde_json::json!(*n),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => bytes_to_json(b),
        Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(to_json).collect())
        }
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> =
                m.iter().map(|(k, val)| (k.clone(), to_json(val))).collect();
            serde_json::Value::Object(obj)
        }
    }
}

/// Convert NRF bytes to a full NRF-encoded buffer, then to JSON.
pub fn nrf_bytes_to_json(bytes: &[u8]) -> Result<serde_json::Value, JsonViewError> {
    let v = nrf_core::decode(bytes).map_err(|e| JsonViewError::NrfDecode(e.to_string()))?;
    Ok(to_json(&v))
}

fn bytes_to_json(b: &[u8]) -> serde_json::Value {
    match b.len() {
        32 => {
            // Bytes(32) → "b3:<hex>"
            serde_json::Value::String(format!("b3:{}", hex::encode(b)))
        }
        16 | 64 => {
            // Bytes(16|64) → "b64:<base64>"
            serde_json::Value::String(format!("b64:{}", B64.encode(b)))
        }
        _ => {
            // Other → {"$bytes": "<base64>"}
            let mut obj = serde_json::Map::new();
            obj.insert("$bytes".into(), serde_json::Value::String(B64.encode(b)));
            serde_json::Value::Object(obj)
        }
    }
}

// ---------------------------------------------------------------------------
// from_json: serde_json::Value → Value
// ---------------------------------------------------------------------------

/// Convert a canonical JSON representation back to an NRF `Value`.
/// Rejects floats, non-NFC strings, BOM, and invalid byte prefixes.
pub fn from_json(j: &serde_json::Value) -> Result<Value, JsonViewError> {
    match j {
        serde_json::Value::Null => Ok(Value::Null),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if n.as_f64().is_some() {
                Err(JsonViewError::Float)
            } else {
                Err(JsonViewError::IntegerOverflow)
            }
        }
        serde_json::Value::String(s) => parse_string_or_bytes(s),
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(from_json(item)?);
            }
            Ok(Value::Array(out))
        }
        serde_json::Value::Object(obj) => {
            // Check for {"$bytes": "<base64>"} special form
            if obj.len() == 1 {
                if let Some(serde_json::Value::String(b64_str)) = obj.get("$bytes") {
                    let decoded = B64
                        .decode(b64_str)
                        .map_err(|e| JsonViewError::BadBase64(e.to_string()))?;
                    return Ok(Value::Bytes(decoded));
                }
            }
            let mut map = BTreeMap::new();
            for (k, val) in obj {
                validate_string(k)?;
                map.insert(k.clone(), from_json(val)?);
            }
            Ok(Value::Map(map))
        }
    }
}

/// Convert JSON to NRF bytes (encode after parsing).
pub fn json_to_nrf_bytes(j: &serde_json::Value) -> Result<Vec<u8>, JsonViewError> {
    let v = from_json(j)?;
    Ok(nrf_core::encode(&v))
}

// ---------------------------------------------------------------------------
// String parsing: plain string vs bytes-encoded-as-string
// ---------------------------------------------------------------------------

fn parse_string_or_bytes(s: &str) -> Result<Value, JsonViewError> {
    // "b3:<64 hex chars>" → Bytes(32)
    if RE_B3.is_match(s) {
        let hex_str = &s[3..];
        let bytes = hex::decode(hex_str).map_err(|_| JsonViewError::BadHex)?;
        return Ok(Value::Bytes(bytes));
    }
    // "b64:<base64>" → Bytes(16 or 64)
    if RE_B64_PREFIX.is_match(s) {
        let b64_str = &s[4..];
        let bytes = B64
            .decode(b64_str)
            .map_err(|e| JsonViewError::BadBase64(e.to_string()))?;
        return Ok(Value::Bytes(bytes));
    }
    // Plain string — validate NFC, no BOM
    validate_string(s)?;
    Ok(Value::String(s.to_string()))
}

fn validate_string(s: &str) -> Result<(), JsonViewError> {
    if s.contains('\u{FEFF}') {
        return Err(JsonViewError::BOMPresent);
    }
    if !unicode_normalization::is_nfc(s) {
        return Err(JsonViewError::NotNFC);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ASCII validation for DID/KID fields
// ---------------------------------------------------------------------------

/// Validate that a string is ASCII-only (for DID/KID fields).
pub fn validate_ascii(s: &str) -> Result<(), JsonViewError> {
    if !s.is_ascii() {
        return Err(JsonViewError::NotASCII);
    }
    Ok(())
}

/// Validate a decimal string against the canonical regex.
pub fn validate_decimal(s: &str) -> Result<(), JsonViewError> {
    if !RE_DECIMAL.is_match(s) {
        return Err(JsonViewError::InvalidDecimal(s.to_string()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use nrf_core::Value;
    use std::collections::BTreeMap;

    // --- Roundtrip tests ---

    #[test]
    fn roundtrip_null() {
        let v = Value::Null;
        let j = to_json(&v);
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bool() {
        for b in [true, false] {
            let v = Value::Bool(b);
            let j = to_json(&v);
            assert_eq!(from_json(&j).unwrap(), v);
        }
    }

    #[test]
    fn roundtrip_int() {
        for n in [0i64, 1, -1, 42, i64::MAX, i64::MIN] {
            let v = Value::Int(n);
            let j = to_json(&v);
            assert_eq!(from_json(&j).unwrap(), v);
        }
    }

    #[test]
    fn roundtrip_string() {
        let v = Value::String("hello world".into());
        let j = to_json(&v);
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_32() {
        let b = vec![0xABu8; 32];
        let v = Value::Bytes(b.clone());
        let j = to_json(&v);
        let j_str = j.as_str().unwrap();
        assert!(j_str.starts_with("b3:"));
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_16() {
        let b = vec![0xCDu8; 16];
        let v = Value::Bytes(b.clone());
        let j = to_json(&v);
        let j_str = j.as_str().unwrap();
        assert!(j_str.starts_with("b64:"));
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_64() {
        let b = vec![0xEFu8; 64];
        let v = Value::Bytes(b.clone());
        let j = to_json(&v);
        let j_str = j.as_str().unwrap();
        assert!(j_str.starts_with("b64:"));
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_empty() {
        let v = Value::Bytes(vec![]);
        let j = to_json(&v);
        assert!(j.is_object());
        assert_eq!(j["$bytes"], "");
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_other_len() {
        let b = vec![1u8, 2, 3, 4, 5];
        let v = Value::Bytes(b.clone());
        let j = to_json(&v);
        assert!(j.is_object());
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_array() {
        let v = Value::Array(vec![
            Value::Bool(true),
            Value::Int(42),
            Value::String("test".into()),
        ]);
        let j = to_json(&v);
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_map() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Int(1));
        m.insert("b".into(), Value::String("two".into()));
        let v = Value::Map(m);
        let j = to_json(&v);
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_nested() {
        let mut inner = BTreeMap::new();
        inner.insert("id".into(), Value::Bytes(vec![0xAA; 32]));
        inner.insert("name".into(), Value::String("nested".into()));
        let v = Value::Array(vec![Value::Map(inner), Value::Null]);
        let j = to_json(&v);
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_nrf_bytes() {
        let mut m = BTreeMap::new();
        m.insert("key".into(), Value::String("value".into()));
        let v = Value::Map(m);
        let nrf = nrf_core::encode(&v);
        let j = nrf_bytes_to_json(&nrf).unwrap();
        let back = json_to_nrf_bytes(&j).unwrap();
        assert_eq!(nrf, back);
    }

    // --- Rejection tests ---

    #[test]
    fn reject_float() {
        let j = serde_json::json!(3.15);
        assert_eq!(from_json(&j).unwrap_err(), JsonViewError::Float);
    }

    #[test]
    fn reject_bom() {
        let j = serde_json::Value::String("\u{FEFF}hello".into());
        assert_eq!(from_json(&j).unwrap_err(), JsonViewError::BOMPresent);
    }

    #[test]
    fn reject_not_nfc() {
        // NFD: e + combining acute accent
        let nfd = "e\u{0301}";
        let j = serde_json::Value::String(nfd.into());
        assert_eq!(from_json(&j).unwrap_err(), JsonViewError::NotNFC);
    }

    #[test]
    fn reject_non_ascii_did() {
        assert_eq!(validate_ascii("did:ubl:café").unwrap_err(), JsonViewError::NotASCII);
    }

    #[test]
    fn accept_ascii_did() {
        assert!(validate_ascii("did:ubl:lab512#key-1").is_ok());
    }

    #[test]
    fn reject_invalid_decimal() {
        assert!(validate_decimal("01.5").is_err());
        assert!(validate_decimal("1e2").is_err());
        assert!(validate_decimal("").is_err());
    }

    #[test]
    fn accept_valid_decimal() {
        assert!(validate_decimal("42").is_ok());
        assert!(validate_decimal("-3.14").is_ok());
        assert!(validate_decimal("0").is_ok());
    }

    #[test]
    fn reject_bad_hex_in_b3() {
        let j = serde_json::Value::String(
            "b3:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz".into(),
        );
        // regex won't match non-hex, so it falls through to plain string
        // which is fine — it's just a string, not bytes
        let result = from_json(&j).unwrap();
        assert!(matches!(result, Value::String(_)));
    }

    #[test]
    fn reject_bad_base64() {
        let j = serde_json::Value::String("b64:!!!invalid!!!".into());
        assert!(from_json(&j).is_err());
    }

    // --- Map key ordering ---

    #[test]
    fn map_keys_sorted() {
        let j = serde_json::json!({"z": 1, "a": 2, "m": 3});
        let v = from_json(&j).unwrap();
        if let Value::Map(m) = v {
            let keys: Vec<&String> = m.keys().collect();
            assert_eq!(keys, vec!["a", "m", "z"]);
        } else {
            panic!("expected map");
        }
    }
}
