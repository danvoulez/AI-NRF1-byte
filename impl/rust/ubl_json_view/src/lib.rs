//! `ai-json-nrf1` — Deterministic JSON view of ai-nrf1 binary values.
//!
//! Provides `to_json` and `from_json` for lossless, canonical round-tripping
//! between `nrf_core::Value` (bytes) and `serde_json::Value` (JSON).
//!
//! Canon 4: The ONLY accepted JSON form for bytes is `{"$bytes": "<lowercase hex>"}`.
//! No base64. No b3: prefix. No b64: prefix. No exceptions.
//!
//! Integers: only Int64 (no floats). Decimals as canonical strings.
//! Strings: NFC, no BOM, ASCII-only enforced for DID/KID fields.

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
    #[error("Err.JsonView.BadBytesObject: {0}")]
    BadBytesObject(String),
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

static RE_HEX_LOWER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[0-9a-f]*$").unwrap());
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
        Value::Array(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> =
                m.iter().map(|(k, val)| (k.clone(), to_json(val))).collect();
            serde_json::Value::Object(obj)
        }
    }
}

/// Convert NRF bytes to a full NRF-encoded buffer, then to JSON.
/// Uses default (conservative) decode limits.
pub fn nrf_bytes_to_json(bytes: &[u8]) -> Result<serde_json::Value, JsonViewError> {
    let v = nrf_core::decode(bytes).map_err(|e| JsonViewError::NrfDecode(e.to_string()))?;
    Ok(to_json(&v))
}

/// Convert NRF bytes to JSON with explicit decode limits.
pub fn nrf_bytes_to_json_with_opts(
    bytes: &[u8],
    opts: &nrf_core::DecodeOpts,
) -> Result<serde_json::Value, JsonViewError> {
    let v = nrf_core::decode_with_opts(bytes, opts)
        .map_err(|e| JsonViewError::NrfDecode(e.to_string()))?;
    Ok(to_json(&v))
}

/// Canon 4: ALL bytes → {"$bytes": "<lowercase hex>"}. No exceptions.
fn bytes_to_json(b: &[u8]) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    obj.insert("$bytes".into(), serde_json::Value::String(nrf_core::encode_hex_lower(b)));
    serde_json::Value::Object(obj)
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
        // Canon 4: strings are just strings. No prefix interpretation.
        serde_json::Value::String(s) => {
            validate_string(s)?;
            Ok(Value::String(s.clone()))
        }
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(from_json(item)?);
            }
            Ok(Value::Array(out))
        }
        serde_json::Value::Object(obj) => {
            // Canon 4: {"$bytes": "<lowercase hex>"} is the ONLY bytes form
            if obj.len() == 1 {
                if let Some(serde_json::Value::String(hex_str)) = obj.get("$bytes") {
                    // Validate: even length, lowercase hex only
                    if hex_str.len() % 2 != 0 {
                        return Err(JsonViewError::OddHex);
                    }
                    if !hex_str.is_empty() && !RE_HEX_LOWER.is_match(hex_str) {
                        return Err(JsonViewError::BadHex);
                    }
                    let decoded = nrf_core::parse_hex_lower(hex_str).map_err(|e| match e {
                        nrf_core::Error::HexOddLength => JsonViewError::OddHex,
                        _ => JsonViewError::BadHex,
                    })?;
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
// Canon vs View type wrappers (item 15)
// ---------------------------------------------------------------------------

/// Canonical ai-nrf1 bytes — the ONLY thing you hash/sign.
/// Wraps encoded NRF bytes (with magic prefix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonBytes(Vec<u8>);

impl CanonBytes {
    /// Encode an NRF Value into canonical bytes.
    pub fn from_value(v: &Value) -> Self {
        Self(nrf_core::encode(v))
    }

    /// Decode canonical bytes back to an NRF Value.
    pub fn to_value(&self) -> Result<Value, JsonViewError> {
        nrf_core::decode(&self.0).map_err(|e| JsonViewError::NrfDecode(e.to_string()))
    }

    /// Raw bytes (for hashing/signing).
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// BLAKE3 hash as `[u8; 32]`.
    pub fn blake3(&self) -> [u8; 32] {
        nrf_core::hash_bytes(&self.0)
    }

    /// BLAKE3 CID as `b3:<hex>`.
    pub fn cid(&self) -> String {
        format!("b3:{}", nrf_core::encode_hex_lower(&self.blake3()))
    }

    /// Convert to JSON view (for display only — never hash this).
    pub fn to_json_view(&self) -> Result<JsonView, JsonViewError> {
        let v = self.to_value()?;
        Ok(JsonView(to_json(&v)))
    }
}

/// JSON view of an NRF value — for display/transport only.
/// NEVER hash or sign this directly; convert back to `CanonBytes` first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonView(serde_json::Value);

impl JsonView {
    /// Parse a JSON value into a view (validates NFC, rejects floats, etc.).
    pub fn from_json(j: &serde_json::Value) -> Result<Self, JsonViewError> {
        // Validate by round-tripping through from_json
        let _ = from_json(j)?;
        Ok(Self(j.clone()))
    }

    /// The underlying serde_json::Value.
    pub fn as_json(&self) -> &serde_json::Value {
        &self.0
    }

    /// Convert back to canonical bytes (the only thing you hash/sign).
    pub fn to_canon_bytes(&self) -> Result<CanonBytes, JsonViewError> {
        let v = from_json(&self.0)?;
        Ok(CanonBytes(nrf_core::encode(&v)))
    }
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
        // Canon 4: always {"$bytes": "<hex>"}
        assert!(j.is_object());
        assert!(j["$bytes"].is_string());
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_16() {
        let b = vec![0xCDu8; 16];
        let v = Value::Bytes(b.clone());
        let j = to_json(&v);
        assert!(j.is_object());
        assert!(j["$bytes"].is_string());
        assert_eq!(from_json(&j).unwrap(), v);
    }

    #[test]
    fn roundtrip_bytes_64() {
        let b = vec![0xEFu8; 64];
        let v = Value::Bytes(b.clone());
        let j = to_json(&v);
        assert!(j.is_object());
        assert!(j["$bytes"].is_string());
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
        assert_eq!(
            validate_ascii("did:ubl:café").unwrap_err(),
            JsonViewError::NotASCII
        );
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
    fn b3_prefix_string_is_just_a_string() {
        // Canon 4: "b3:..." is a plain string, NOT bytes
        let j = serde_json::Value::String(
            "b3:0000000000000000000000000000000000000000000000000000000000000000".into(),
        );
        let result = from_json(&j).unwrap();
        assert!(matches!(result, Value::String(_)));
    }

    #[test]
    fn b64_prefix_string_is_just_a_string() {
        // Canon 4: "b64:..." is a plain string, NOT bytes
        let j = serde_json::Value::String("b64:AAAA".into());
        let result = from_json(&j).unwrap();
        assert!(matches!(result, Value::String(_)));
    }

    #[test]
    fn reject_bad_hex_in_bytes_object() {
        let j = serde_json::json!({"$bytes": "ZZZZ"});
        assert!(from_json(&j).is_err());
    }

    #[test]
    fn reject_uppercase_hex_in_bytes_object() {
        let j = serde_json::json!({"$bytes": "DEADBEEF"});
        assert!(from_json(&j).is_err());
    }

    #[test]
    fn reject_odd_hex_in_bytes_object() {
        let j = serde_json::json!({"$bytes": "abc"});
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

    // --- Canon vs View type wrapper tests ---

    #[test]
    fn canon_bytes_roundtrip() {
        let mut m = BTreeMap::new();
        m.insert("key".into(), Value::String("value".into()));
        let v = Value::Map(m);
        let canon = CanonBytes::from_value(&v);
        let back = canon.to_value().unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn canon_bytes_cid_format() {
        let v = Value::String("hello".into());
        let canon = CanonBytes::from_value(&v);
        let cid = canon.cid();
        assert!(cid.starts_with("b3:"));
        assert_eq!(cid.len(), 3 + 64); // "b3:" + 64 hex chars
    }

    #[test]
    fn canon_to_json_view_roundtrip() {
        let v = Value::Int(42);
        let canon = CanonBytes::from_value(&v);
        let view = canon.to_json_view().unwrap();
        let back = view.to_canon_bytes().unwrap();
        assert_eq!(canon, back);
    }

    #[test]
    fn json_view_rejects_float() {
        let j = serde_json::json!(3.15);
        assert!(JsonView::from_json(&j).is_err());
    }

    #[test]
    fn json_view_accepts_valid() {
        let j = serde_json::json!({"name": "test", "count": 42});
        let view = JsonView::from_json(&j).unwrap();
        let canon = view.to_canon_bytes().unwrap();
        let back_view = canon.to_json_view().unwrap();
        assert_eq!(view.as_json()["name"], back_view.as_json()["name"]);
    }
}
