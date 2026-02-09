// ---------------------------------------------------------------------------
// ρ (rho) — The Semantic Normalization Engine
//
// ρ is the policy engine at the byte level. It is not a pre-processing step.
// It is not a convenience. It is the LAW.
//
// Before any value is encoded, before any hash is computed, before any
// signature is applied — ρ rewrites the value into its ONE canonical form.
//
// Properties:
//   ρ(ρ(v)) = ρ(v)                    — idempotent
//   encode(ρ(v)) is the ONLY valid encoding of v
//   BLAKE3(encode(ρ(v))) is the ONLY valid hash of v
//
// Rules:
//   1. Strings     → UTF-8, NFC normalized, no BOM (U+FEFF)
//   2. Timestamps  → RFC-3339 UTC 'Z', minimal fractional seconds
//   3. Decimals    → No exponent, no leading zeros, no superfluous .0
//   4. Sets        → Sort by canonical NRF bytes of each element, deduplicate
//   5. Maps        → Keys sorted (BTreeMap guarantees this), null values REMOVED
//   6. Arrays      → Elements recursively normalized
//   7. Null/Bool/Int/Bytes → Pass through (already canonical)
//
// ρ is recursive. It normalizes the leaves, then the containers.
// ---------------------------------------------------------------------------

use crate::Value;
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;
use regex::Regex;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Error type for ρ violations that cannot be auto-fixed
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RhoError {
    InvalidUTF8,
    InvalidDecimal(String),
    InvalidTimestamp(String),
}

impl std::fmt::Display for RhoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidUTF8 => write!(f, "Rho.InvalidUTF8"),
            Self::InvalidDecimal(s) => write!(f, "Rho.InvalidDecimal({s})"),
            Self::InvalidTimestamp(s) => write!(f, "Rho.InvalidTimestamp({s})"),
        }
    }
}

// ---------------------------------------------------------------------------
// Regex patterns (compiled once)
// ---------------------------------------------------------------------------

static RE_DECIMAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?$").unwrap()
});

static RE_TIMESTAMP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$").unwrap()
});

// ---------------------------------------------------------------------------
// The ρ function — the heart of the system
// ---------------------------------------------------------------------------

/// Apply ρ normalization to a Value. Returns the canonical form.
/// This is the policy engine at the byte level.
///
/// After ρ, the value is guaranteed to produce a single deterministic
/// byte stream via encode(), a single deterministic hash via BLAKE3,
/// and a single deterministic signature via Ed25519.
pub fn normalize(v: &Value) -> Result<Value, RhoError> {
    match v {
        Value::Null => Ok(Value::Null),
        Value::Bool(b) => Ok(Value::Bool(*b)),
        Value::Int(n) => Ok(Value::Int(*n)),
        Value::Bytes(b) => Ok(Value::Bytes(b.clone())),

        Value::String(s) => normalize_string(s),

        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(normalize(item)?);
            }
            Ok(Value::Array(out))
        }

        Value::Map(m) => {
            let mut out = BTreeMap::new();
            for (k, val) in m {
                // ρ rule 1: normalize the key string (NFC, no BOM)
                let norm_key = normalize_string_raw(k)?;
                // ρ rule 5: null values are REMOVED (absence ≠ null)
                let norm_val = normalize(val)?;
                if norm_val != Value::Null {
                    out.insert(norm_key, norm_val);
                }
            }
            Ok(Value::Map(out))
        }
    }
}

/// Normalize and encode for canonical hashing.
/// This is the blessed path: ρ → encode → BLAKE3 → CID.
pub fn canonical_encode(v: &Value) -> Result<Vec<u8>, RhoError> {
    let normalized = normalize(v)?;
    Ok(crate::encode(&normalized))
}

/// Normalize, encode, and hash. The ONE canonical CID.
pub fn canonical_cid(v: &Value) -> Result<String, RhoError> {
    let bytes = canonical_encode(v)?;
    let hash = blake3::hash(&bytes);
    Ok(format!("b3:{}", hash.to_hex()))
}

/// Sort an array as a Set: sort by canonical NRF bytes, deduplicate.
/// This is ρ rule 4 — applied explicitly when the caller knows the
/// array represents a set (e.g., evidence CIDs, policy references).
pub fn normalize_as_set(items: &[Value]) -> Result<Vec<Value>, RhoError> {
    let mut pairs: Vec<(Vec<u8>, Value)> = Vec::with_capacity(items.len());
    for item in items {
        let norm = normalize(item)?;
        let bytes = crate::encode(&norm);
        pairs.push((bytes, norm));
    }
    // Sort by canonical NRF bytes
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    // Deduplicate adjacent equal byte representations
    pairs.dedup_by(|a, b| a.0 == b.0);
    Ok(pairs.into_iter().map(|(_, v)| v).collect())
}

// ---------------------------------------------------------------------------
// String normalization (ρ rules 1, 2, 3)
// ---------------------------------------------------------------------------

fn normalize_string(s: &str) -> Result<Value, RhoError> {
    Ok(Value::String(normalize_string_raw(s)?))
}

fn normalize_string_raw(s: &str) -> Result<String, RhoError> {
    // ρ rule 1a: NFC normalize
    let nfc: String = s.nfc().collect();

    // ρ rule 1b: reject BOM (U+FEFF) — do not silently strip
    if nfc.contains('\u{FEFF}') {
        return Err(RhoError::InvalidUTF8);
    }

    Ok(nfc)
}

// ---------------------------------------------------------------------------
// Timestamp normalization (ρ rule 2)
// ---------------------------------------------------------------------------

/// Normalize a timestamp string to canonical form:
/// RFC-3339 UTC 'Z', minimal fractional seconds, no .000
///
/// "2024-01-15T10:30:00.000Z" → "2024-01-15T10:30:00Z"
/// "2024-01-15T10:30:00.100Z" → "2024-01-15T10:30:00.1Z"
/// "2024-01-15T10:30:00Z"     → "2024-01-15T10:30:00Z" (unchanged)
pub fn normalize_timestamp(s: &str) -> Result<String, RhoError> {
    if !RE_TIMESTAMP.is_match(s) {
        return Err(RhoError::InvalidTimestamp(s.to_string()));
    }

    // Strip trailing zeros from fractional part, and the dot if fraction is all zeros
    if let Some(dot_pos) = s.rfind('.') {
        let before_dot = &s[..dot_pos];
        let frac_and_z = &s[dot_pos + 1..]; // e.g. "100Z" or "000Z"
        let frac = &frac_and_z[..frac_and_z.len() - 1]; // strip 'Z'
        let trimmed = frac.trim_end_matches('0');
        if trimmed.is_empty() {
            // All zeros — drop the fraction entirely
            Ok(format!("{before_dot}Z"))
        } else {
            Ok(format!("{before_dot}.{trimmed}Z"))
        }
    } else {
        // No fractional part — already canonical
        Ok(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Decimal normalization (ρ rule 3)
// ---------------------------------------------------------------------------

/// Normalize a decimal string to canonical form:
/// No exponent, no leading zeros, no superfluous .0
///
/// "1.0"    → "1"
/// "01.5"   → rejected (leading zero)
/// "1.50"   → "1.5"
/// "-0.0"   → "0"
/// "1e2"    → rejected (exponent)
pub fn normalize_decimal(s: &str) -> Result<String, RhoError> {
    if !RE_DECIMAL.is_match(s) {
        return Err(RhoError::InvalidDecimal(s.to_string()));
    }

    let is_negative = s.starts_with('-');
    let abs = if is_negative { &s[1..] } else { s };

    if let Some(dot_pos) = abs.find('.') {
        let int_part = &abs[..dot_pos];
        let frac_part = abs[dot_pos + 1..].trim_end_matches('0');

        if frac_part.is_empty() {
            // e.g. "1.0" → "1", "-0.0" → "0"
            let result = int_part.to_string();
            if is_negative && result != "0" {
                Ok(format!("-{result}"))
            } else {
                Ok(result)
            }
        } else {
            let result = format!("{int_part}.{frac_part}");
            if is_negative {
                Ok(format!("-{result}"))
            } else {
                Ok(result)
            }
        }
    } else {
        // No decimal point — already canonical
        if is_negative && abs == "0" {
            Ok("0".to_string()) // -0 → 0
        } else {
            Ok(s.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// Validate — strict mode. Returns errors instead of rewriting.
// Use this when you want to REJECT non-canonical input instead of fixing it.
// ---------------------------------------------------------------------------

/// Validate that a value is already in ρ-normal form.
/// Returns Ok(()) if canonical, Err with the first violation found.
pub fn validate(v: &Value) -> Result<(), RhoError> {
    let normalized = normalize(v)?;
    if &normalized != v {
        // The value changed under ρ — it was not canonical
        // Find the specific violation for a useful error message
        if let (Value::Map(orig), Value::Map(norm)) = (v, &normalized) {
            // Check for null values that should have been stripped
            for (k, val) in orig {
                if *val == Value::Null && !norm.contains_key(k) {
                    return Err(RhoError::InvalidDecimal(
                        format!("map key '{k}' has null value (absence ≠ null)"),
                    ));
                }
            }
        }
        Err(RhoError::InvalidDecimal("value is not in ρ-normal form".to_string()))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_null_passthrough() {
        assert_eq!(normalize(&Value::Null).unwrap(), Value::Null);
    }

    #[test]
    fn test_normalize_int_passthrough() {
        assert_eq!(normalize(&Value::Int(42)).unwrap(), Value::Int(42));
    }

    #[test]
    fn test_normalize_string_nfc() {
        // é as combining sequence (e + combining acute) → NFC é
        let decomposed = "e\u{0301}"; // NFD
        let result = normalize(&Value::String(decomposed.to_string())).unwrap();
        assert_eq!(result, Value::String("\u{00E9}".to_string())); // NFC é
    }

    #[test]
    fn test_normalize_string_bom_rejected() {
        let with_bom = "\u{FEFF}hello";
        assert!(normalize(&Value::String(with_bom.to_string())).is_err());
    }

    #[test]
    fn test_normalize_map_strips_nulls() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Int(1));
        m.insert("b".into(), Value::Null);
        m.insert("c".into(), Value::Int(3));
        let result = normalize(&Value::Map(m)).unwrap();
        if let Value::Map(rm) = result {
            assert_eq!(rm.len(), 2);
            assert!(!rm.contains_key("b"));
        } else {
            panic!("expected map");
        }
    }

    #[test]
    fn test_normalize_idempotent() {
        let mut m = BTreeMap::new();
        m.insert("x".into(), Value::String("e\u{0301}".to_string()));
        m.insert("y".into(), Value::Null);
        let v = Value::Map(m);
        let r1 = normalize(&v).unwrap();
        let r2 = normalize(&r1).unwrap();
        assert_eq!(r1, r2); // ρ(ρ(v)) = ρ(v)
    }

    #[test]
    fn test_timestamp_strip_zero_fraction() {
        assert_eq!(
            normalize_timestamp("2024-01-15T10:30:00.000Z").unwrap(),
            "2024-01-15T10:30:00Z"
        );
    }

    #[test]
    fn test_timestamp_minimal_fraction() {
        assert_eq!(
            normalize_timestamp("2024-01-15T10:30:00.100Z").unwrap(),
            "2024-01-15T10:30:00.1Z"
        );
    }

    #[test]
    fn test_timestamp_already_canonical() {
        assert_eq!(
            normalize_timestamp("2024-01-15T10:30:00Z").unwrap(),
            "2024-01-15T10:30:00Z"
        );
    }

    #[test]
    fn test_timestamp_invalid_rejected() {
        assert!(normalize_timestamp("2024-01-15 10:30:00").is_err());
        assert!(normalize_timestamp("not-a-timestamp").is_err());
    }

    #[test]
    fn test_decimal_strip_trailing_zero() {
        assert_eq!(normalize_decimal("1.50").unwrap(), "1.5");
    }

    #[test]
    fn test_decimal_strip_dot_zero() {
        assert_eq!(normalize_decimal("1.0").unwrap(), "1");
    }

    #[test]
    fn test_decimal_negative_zero() {
        assert_eq!(normalize_decimal("-0").unwrap(), "0");
        assert_eq!(normalize_decimal("-0.0").unwrap(), "0");
    }

    #[test]
    fn test_decimal_reject_leading_zero() {
        assert!(normalize_decimal("01.5").is_err());
    }

    #[test]
    fn test_decimal_reject_exponent() {
        assert!(normalize_decimal("1e2").is_err());
        assert!(normalize_decimal("1.5E10").is_err());
    }

    #[test]
    fn test_decimal_already_canonical() {
        assert_eq!(normalize_decimal("42").unwrap(), "42");
        assert_eq!(normalize_decimal("-3.14").unwrap(), "-3.14");
    }

    #[test]
    fn test_set_sort_and_dedup() {
        let items = vec![
            Value::String("b".into()),
            Value::String("a".into()),
            Value::String("b".into()), // duplicate
        ];
        let result = normalize_as_set(&items).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], Value::String("a".into()));
        assert_eq!(result[1], Value::String("b".into()));
    }

    #[test]
    fn test_canonical_cid_deterministic() {
        let mut m = BTreeMap::new();
        m.insert("key".into(), Value::String("value".into()));
        let v = Value::Map(m);
        let c1 = canonical_cid(&v).unwrap();
        let c2 = canonical_cid(&v).unwrap();
        assert_eq!(c1, c2);
        assert!(c1.starts_with("b3:"));
    }

    #[test]
    fn test_validate_catches_non_canonical() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Null); // should be stripped
        m.insert("b".into(), Value::Int(1));
        assert!(validate(&Value::Map(m)).is_err());
    }

    #[test]
    fn test_validate_passes_canonical() {
        let mut m = BTreeMap::new();
        m.insert("b".into(), Value::Int(1));
        assert!(validate(&Value::Map(m)).is_ok());
    }
}
