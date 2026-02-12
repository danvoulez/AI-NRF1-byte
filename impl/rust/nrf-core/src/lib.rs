use std::collections::BTreeMap;
use std::io;

pub mod rho;

/// Magic "nrf1"
pub const MAGIC: [u8; 4] = *b"nrf1";

/// MIME types (from UBL Capsule v1 spec)
pub const MIME_NRF: &str = "application/ai-nrf1";
pub const MIME_JSON: &str = "application/ai-json-nrf1+json";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("InvalidMagic")]
    InvalidMagic,
    #[error("InvalidTypeTag({0:#04x})")]
    InvalidTypeTag(u8),
    #[error("NonMinimalVarint")]
    NonMinimalVarint,
    #[error("UnexpectedEOF")]
    UnexpectedEOF,
    #[error("InvalidUTF8")]
    InvalidUTF8,
    #[error("NotNFC")]
    NotNFC,
    #[error("BOMPresent")]
    BOMPresent,
    #[error("NonStringKey")]
    NonStringKey,
    #[error("UnsortedKeys")]
    UnsortedKeys,
    #[error("DuplicateKey")]
    DuplicateKey,
    #[error("TrailingData")]
    TrailingData,
    #[error("DepthExceeded")]
    DepthExceeded,
    #[error("SizeExceeded")]
    SizeExceeded,
    #[error("StringTooLong")]
    StringTooLong,
    #[error("BytesTooLong")]
    BytesTooLong,
    #[error("ArrayTooLong")]
    ArrayTooLong,
    #[error("MapTooLong")]
    MapTooLong,
    #[error("Io({0})")]
    Io(String),
    #[error("HexOddLength")]
    HexOddLength,
    #[error("HexUppercase")]
    HexUppercase,
    #[error("HexInvalidChar")]
    HexInvalidChar,
    #[error("NotASCII")]
    NotASCII,
    #[error("Float")]
    Float,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            Error::UnexpectedEOF
        } else {
            Error::Io(e.to_string())
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Encode value with magic prefix.
#[cfg_attr(feature = "obs", tracing::instrument(level = "trace", skip_all))]
pub fn encode(value: &Value) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&MAGIC);
    encode_value(&mut buf, value);
    buf
}

fn encode_value(buf: &mut Vec<u8>, v: &Value) {
    match v {
        Value::Null => buf.push(0x00),
        Value::Bool(false) => buf.push(0x01),
        Value::Bool(true) => buf.push(0x02),
        Value::Int(n) => {
            buf.push(0x03);
            buf.extend_from_slice(&n.to_be_bytes());
        }
        Value::String(s) => {
            buf.push(0x04);
            encode_varint32(buf, s.len() as u32);
            buf.extend_from_slice(s.as_bytes());
        }
        Value::Bytes(b) => {
            buf.push(0x05);
            encode_varint32(buf, b.len() as u32);
            buf.extend_from_slice(b);
        }
        Value::Array(items) => {
            buf.push(0x06);
            encode_varint32(buf, items.len() as u32);
            for it in items {
                encode_value(buf, it);
            }
        }
        Value::Map(m) => {
            buf.push(0x07);
            encode_varint32(buf, m.len() as u32);
            for (k, val) in m {
                // key is encoded as String
                buf.push(0x04);
                encode_varint32(buf, k.len() as u32);
                buf.extend_from_slice(k.as_bytes());
                encode_value(buf, val);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Decode options — Canon 6: resource limits against DoS
// ---------------------------------------------------------------------------

/// Resource limits for decoding. Canon 6 / Security Considerations §3.2.
#[derive(Debug, Clone)]
pub struct DecodeOpts {
    /// Maximum nesting depth for arrays/maps (default: 64).
    pub max_depth: usize,
    /// Maximum total input bytes (default: 8 MiB).
    pub max_total_bytes: usize,
    /// Maximum byte length of a single string (default: 1 MiB).
    pub max_string_len: usize,
    /// Maximum byte length of a single Bytes value (default: 1 MiB).
    pub max_bytes_len: usize,
    /// Maximum number of elements in a single array (default: 100_000).
    pub max_array_len: usize,
    /// Maximum number of pairs in a single map (default: 100_000).
    pub max_map_len: usize,
}

impl Default for DecodeOpts {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_total_bytes: 8 * 1024 * 1024, // 8 MiB
            max_string_len: 1024 * 1024,       // 1 MiB
            max_bytes_len: 1024 * 1024,        // 1 MiB
            max_array_len: 100_000,
            max_map_len: 100_000,
        }
    }
}

impl DecodeOpts {
    /// Permissive opts for trusted internal use (e.g. re-decoding our own output).
    pub fn permissive() -> Self {
        Self {
            max_depth: 256,
            max_total_bytes: usize::MAX,
            max_string_len: usize::MAX,
            max_bytes_len: usize::MAX,
            max_array_len: usize::MAX,
            max_map_len: usize::MAX,
        }
    }
}

/// Decode from full buffer (with magic) and reject trailing bytes.
/// Uses default (conservative) resource limits.
#[cfg_attr(feature = "obs", tracing::instrument(level = "trace", skip_all, fields(len = data.len())))]
pub fn decode(data: &[u8]) -> Result<Value> {
    decode_with_opts(data, &DecodeOpts::default())
}

/// Decode with explicit resource limits. Canon 6: reject, never degrade.
#[cfg_attr(feature = "obs", tracing::instrument(level = "trace", skip_all, fields(len = data.len())))]
pub fn decode_with_opts(data: &[u8], opts: &DecodeOpts) -> Result<Value> {
    if data.len() > opts.max_total_bytes {
        return Err(Error::SizeExceeded);
    }
    if data.len() < 4 {
        return Err(Error::InvalidMagic);
    }
    if data[..4] != MAGIC {
        return Err(Error::InvalidMagic);
    }
    let mut cur = &data[4..];
    let v = decode_value_opts(&mut cur, 0, opts)?;
    if !cur.is_empty() {
        return Err(Error::TrailingData);
    }
    Ok(v)
}

fn decode_value_opts(cur: &mut &[u8], depth: usize, opts: &DecodeOpts) -> Result<Value> {
    if depth > opts.max_depth {
        return Err(Error::DepthExceeded);
    }
    if cur.is_empty() {
        return Err(Error::UnexpectedEOF);
    }
    let tag = cur[0];
    *cur = &cur[1..];
    match tag {
        0x00 => Ok(Value::Null),
        0x01 => Ok(Value::Bool(false)),
        0x02 => Ok(Value::Bool(true)),
        0x03 => {
            if cur.len() < 8 {
                return Err(Error::UnexpectedEOF);
            }
            let (num, rest) = cur.split_at(8);
            *cur = rest;
            let mut arr = [0u8; 8];
            arr.copy_from_slice(num);
            Ok(Value::Int(i64::from_be_bytes(arr)))
        }
        0x04 => {
            let len = decode_varint32(cur)? as usize;
            if len > opts.max_string_len {
                return Err(Error::StringTooLong);
            }
            if cur.len() < len {
                return Err(Error::UnexpectedEOF);
            }
            let (bytes, rest) = cur.split_at(len);
            *cur = rest;
            let s = std::str::from_utf8(bytes).map_err(|_| Error::InvalidUTF8)?;
            if s.chars().any(|c| c == '\u{FEFF}') {
                return Err(Error::BOMPresent);
            }
            if !unicode_normalization::is_nfc(s) {
                return Err(Error::NotNFC);
            }
            Ok(Value::String(s.to_string()))
        }
        0x05 => {
            let len = decode_varint32(cur)? as usize;
            if len > opts.max_bytes_len {
                return Err(Error::BytesTooLong);
            }
            if cur.len() < len {
                return Err(Error::UnexpectedEOF);
            }
            let (bytes, rest) = cur.split_at(len);
            *cur = rest;
            Ok(Value::Bytes(bytes.to_vec()))
        }
        0x06 => {
            let count = decode_varint32(cur)? as usize;
            if count > opts.max_array_len {
                return Err(Error::ArrayTooLong);
            }
            let mut v = Vec::with_capacity(count.min(1024));
            for _ in 0..count {
                v.push(decode_value_opts(cur, depth + 1, opts)?);
            }
            Ok(Value::Array(v))
        }
        0x07 => {
            let count = decode_varint32(cur)? as usize;
            if count > opts.max_map_len {
                return Err(Error::MapTooLong);
            }
            let mut map = BTreeMap::new();
            let mut prev: Option<Vec<u8>> = None;
            for _ in 0..count {
                // key must be a string (0x04)
                if cur.is_empty() {
                    return Err(Error::UnexpectedEOF);
                }
                let key_tag = cur[0];
                *cur = &cur[1..];
                if key_tag != 0x04 {
                    return Err(Error::NonStringKey);
                }
                let klen = decode_varint32(cur)? as usize;
                if klen > opts.max_string_len {
                    return Err(Error::StringTooLong);
                }
                if cur.len() < klen {
                    return Err(Error::UnexpectedEOF);
                }
                let (kbytes, rest) = cur.split_at(klen);
                *cur = rest;
                let kstr = std::str::from_utf8(kbytes).map_err(|_| Error::InvalidUTF8)?;
                if kstr.chars().any(|c| c == '\u{FEFF}') {
                    return Err(Error::BOMPresent);
                }
                if !unicode_normalization::is_nfc(kstr) {
                    return Err(Error::NotNFC);
                }
                if let Some(prevb) = prev.as_ref() {
                    match prevb.as_slice().cmp(kbytes) {
                        std::cmp::Ordering::Less => {}
                        std::cmp::Ordering::Equal => return Err(Error::DuplicateKey),
                        std::cmp::Ordering::Greater => return Err(Error::UnsortedKeys),
                    }
                }
                prev = Some(kbytes.to_vec());
                let val = decode_value_opts(cur, depth + 1, opts)?;
                map.insert(kstr.to_string(), val);
            }
            Ok(Value::Map(map))
        }
        _ => Err(Error::InvalidTypeTag(tag)),
    }
}

#[cfg_attr(feature = "obs", tracing::instrument(level = "trace", skip_all, fields(len = data.len())))]
pub fn hash_bytes(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

#[cfg_attr(feature = "obs", tracing::instrument(level = "trace", skip_all))]
pub fn hash_value(v: &Value) -> [u8; 32] {
    let bytes = encode(v);
    hash_bytes(&bytes)
}

/// Alias for `encode` — compat with crates/nrf1 API.
pub fn encode_stream(value: &Value) -> Vec<u8> {
    encode(value)
}

/// Compute BLAKE3 CID string from a Value: `b3:<hex>`.
pub fn blake3_cid(value: &Value) -> String {
    let bytes = encode(value);
    let hash = blake3::hash(&bytes);
    format!("b3:{}", hash.to_hex())
}

/// Type alias for backward compat with crates/nrf1.
pub type NrfError = Error;

// ---------------------------------------------------------------------------
// Canonical hex utilities (shared across all crates)
// ---------------------------------------------------------------------------

/// Parse a lowercase hex string into bytes.
/// Rejects: odd length, uppercase, non-hex chars.
/// Accepts empty string → empty vec.
pub fn parse_hex_lower(s: &str) -> Result<Vec<u8>> {
    if s.is_empty() {
        return Ok(Vec::new());
    }
    if s.len() % 2 != 0 {
        return Err(Error::HexOddLength);
    }
    for ch in s.chars() {
        if !ch.is_ascii_hexdigit() {
            return Err(Error::HexInvalidChar);
        }
        if ch.is_ascii_uppercase() {
            return Err(Error::HexUppercase);
        }
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let b = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| Error::HexInvalidChar)?;
        out.push(b);
    }
    Ok(out)
}

/// Encode bytes as lowercase hex.
pub fn encode_hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Validate that a string is ASCII-only (for DID/KID fields).
pub fn validate_ascii(s: &str) -> Result<()> {
    if s.is_ascii() {
        Ok(())
    } else {
        Err(Error::NotASCII)
    }
}

/// Validate that a string is NFC-normalized and has no BOM.
pub fn validate_nfc(s: &str) -> Result<()> {
    if s.contains('\u{FEFF}') {
        return Err(Error::BOMPresent);
    }
    if !unicode_normalization::is_nfc(s) {
        return Err(Error::NotNFC);
    }
    Ok(())
}

// ----- varint32 (unsigned LEB128 minimal) -----

fn decode_varint32(cur: &mut &[u8]) -> Result<u32> {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    for i in 0..5 {
        if cur.is_empty() {
            return Err(Error::UnexpectedEOF);
        }
        let byte = cur[0];
        *cur = &cur[1..];
        let payload = (byte & 0x7F) as u32;

        // First byte 0x80 => Non-minimal (0 with continuation)
        if i == 0 && byte == 0x80 {
            return Err(Error::NonMinimalVarint);
        }
        // Disallow 0x00 as a continuation byte (leading zero in base-128)
        if i > 0 && byte == 0x00 {
            return Err(Error::NonMinimalVarint);
        }

        result |= payload << shift;
        if (byte & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
    }
    Err(Error::NonMinimalVarint)
}

fn encode_varint32(buf: &mut Vec<u8>, mut value: u32) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            buf.push(byte);
            break;
        } else {
            buf.push(byte | 0x80);
        }
    }
}

// --- Fuzz-only exposure (feature: fuzz_expose) ---
#[cfg(feature = "fuzz_expose")]
pub fn _fuzz_decode_varint32(bytes: &[u8]) -> Result<u32> {
    let mut slice = bytes;
    // decode_varint32 takes a `&mut &[u8]`
    decode_varint32(&mut slice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_simple() {
        let mut map = BTreeMap::new();
        map.insert("name".into(), Value::String("test".into()));
        map.insert("value".into(), Value::Int(42));
        let v = Value::Map(map);
        let enc = encode(&v);
        let dec = decode(&enc).unwrap();
        assert_eq!(v, dec);
        assert_eq!(enc, encode(&dec));
    }

    // --- Hex canon util KATs ---

    #[test]
    fn hex_lower_roundtrip() {
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let hex = encode_hex_lower(&bytes);
        assert_eq!(hex, "deadbeef");
        assert_eq!(parse_hex_lower(&hex).unwrap(), bytes);
    }

    #[test]
    fn hex_empty() {
        assert_eq!(parse_hex_lower("").unwrap(), Vec::<u8>::new());
        assert_eq!(encode_hex_lower(&[]), "");
    }

    #[test]
    fn hex_reject_uppercase() {
        assert_eq!(
            parse_hex_lower("DEADBEEF").unwrap_err(),
            Error::HexUppercase
        );
    }

    #[test]
    fn hex_reject_odd_length() {
        assert_eq!(parse_hex_lower("abc").unwrap_err(), Error::HexOddLength);
    }

    #[test]
    fn hex_reject_invalid_char() {
        assert_eq!(parse_hex_lower("zz").unwrap_err(), Error::HexInvalidChar);
    }

    #[test]
    fn hex_reject_mixed_case() {
        assert_eq!(parse_hex_lower("aAbB").unwrap_err(), Error::HexUppercase);
    }

    // --- Bytes empty KAT ---

    #[test]
    fn roundtrip_bytes_empty() {
        let v = Value::Bytes(vec![]);
        let enc = encode(&v);
        let dec = decode(&enc).unwrap();
        assert_eq!(v, dec);
        // Verify exact encoding: magic + tag 0x05 + varint 0
        assert_eq!(enc, vec![0x6e, 0x72, 0x66, 0x31, 0x05, 0x00]);
    }

    // --- NFC vs NFD key ordering ---

    #[test]
    fn reject_nfd_key() {
        // NFD: "e" + combining acute = é (two codepoints)
        let nfd_key = "e\u{0301}";
        let mut map = BTreeMap::new();
        map.insert(nfd_key.to_string(), Value::Int(1));
        let v = Value::Map(map);
        let enc = encode(&v);
        // Decoder must reject NFD keys
        assert_eq!(decode(&enc).unwrap_err(), Error::NotNFC);
    }

    #[test]
    fn accept_nfc_key() {
        // NFC: "é" (single codepoint)
        let nfc_key = "\u{00E9}";
        let mut map = BTreeMap::new();
        map.insert(nfc_key.to_string(), Value::Int(1));
        let v = Value::Map(map);
        let enc = encode(&v);
        assert_eq!(decode(&enc).unwrap(), v);
    }

    // --- Map key ordering by bytes ---

    #[test]
    fn map_keys_sorted_by_bytes() {
        // "a" < "b" < "é" (0xC3 0xA9) in byte order
        let mut map = BTreeMap::new();
        map.insert("b".into(), Value::Int(2));
        map.insert("a".into(), Value::Int(1));
        map.insert("\u{00E9}".into(), Value::Int(3)); // é
        let v = Value::Map(map);
        let enc = encode(&v);
        let dec = decode(&enc).unwrap();
        if let Value::Map(m) = &dec {
            let keys: Vec<&String> = m.keys().collect();
            assert_eq!(keys, vec!["a", "b", "\u{00E9}"]);
        }
    }

    // --- Validate ASCII/NFC utils ---

    #[test]
    fn validate_ascii_ok() {
        assert!(validate_ascii("did:ubl:lab512#key-1").is_ok());
    }

    #[test]
    fn validate_ascii_reject() {
        assert_eq!(validate_ascii("did:ubl:café").unwrap_err(), Error::NotASCII);
    }

    #[test]
    fn validate_nfc_ok() {
        assert!(validate_nfc("hello").is_ok());
        assert!(validate_nfc("\u{00E9}").is_ok()); // NFC é
    }

    #[test]
    fn validate_nfc_reject_bom() {
        assert_eq!(
            validate_nfc("\u{FEFF}hello").unwrap_err(),
            Error::BOMPresent
        );
    }

    #[test]
    fn validate_nfc_reject_nfd() {
        assert_eq!(validate_nfc("e\u{0301}").unwrap_err(), Error::NotNFC);
    }

    // --- DecodeOpts: malicious corpus tests (Canon 6 DoS guard) ---

    #[test]
    fn decode_opts_default_roundtrip() {
        let v = Value::String("hello".into());
        let enc = encode(&v);
        assert_eq!(decode_with_opts(&enc, &DecodeOpts::default()).unwrap(), v);
    }

    #[test]
    fn decode_opts_rejects_oversized_input() {
        let opts = DecodeOpts { max_total_bytes: 10, ..DecodeOpts::default() };
        // magic(4) + tag(1) + varint(1) + "hello"(5) = 11 bytes > 10
        let v = Value::String("hello".into());
        let enc = encode(&v);
        assert!(enc.len() > 10);
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::SizeExceeded);
    }

    #[test]
    fn decode_opts_rejects_deep_nesting() {
        // Build array nested 5 deep, then decode with max_depth=3
        let mut v = Value::Int(1);
        for _ in 0..5 {
            v = Value::Array(vec![v]);
        }
        let enc = encode(&v);
        let opts = DecodeOpts { max_depth: 3, ..DecodeOpts::default() };
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::DepthExceeded);
    }

    #[test]
    fn decode_opts_accepts_within_depth() {
        let mut v = Value::Int(1);
        for _ in 0..3 {
            v = Value::Array(vec![v]);
        }
        let enc = encode(&v);
        let opts = DecodeOpts { max_depth: 5, ..DecodeOpts::default() };
        assert!(decode_with_opts(&enc, &opts).is_ok());
    }

    #[test]
    fn decode_opts_rejects_long_string() {
        let s = "a".repeat(1000);
        let v = Value::String(s);
        let enc = encode(&v);
        let opts = DecodeOpts { max_string_len: 100, ..DecodeOpts::default() };
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::StringTooLong);
    }

    #[test]
    fn decode_opts_rejects_long_bytes() {
        let b = vec![0xAA; 1000];
        let v = Value::Bytes(b);
        let enc = encode(&v);
        let opts = DecodeOpts { max_bytes_len: 100, ..DecodeOpts::default() };
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::BytesTooLong);
    }

    #[test]
    fn decode_opts_rejects_large_array() {
        let v = Value::Array(vec![Value::Null; 200]);
        let enc = encode(&v);
        let opts = DecodeOpts { max_array_len: 100, ..DecodeOpts::default() };
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::ArrayTooLong);
    }

    #[test]
    fn decode_opts_rejects_large_map() {
        let mut m = BTreeMap::new();
        for i in 0..200 {
            m.insert(format!("k{i:04}"), Value::Null);
        }
        let v = Value::Map(m);
        let enc = encode(&v);
        let opts = DecodeOpts { max_map_len: 100, ..DecodeOpts::default() };
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::MapTooLong);
    }

    #[test]
    fn decode_opts_rejects_long_map_key() {
        let mut m = BTreeMap::new();
        m.insert("a".repeat(500), Value::Int(1));
        let v = Value::Map(m);
        let enc = encode(&v);
        let opts = DecodeOpts { max_string_len: 100, ..DecodeOpts::default() };
        assert_eq!(decode_with_opts(&enc, &opts).unwrap_err(), Error::StringTooLong);
    }

    #[test]
    fn decode_opts_permissive_accepts_large() {
        let v = Value::Array(vec![Value::Null; 200_000]);
        let enc = encode(&v);
        // Default would reject (max_array_len=100k), permissive accepts
        assert_eq!(decode_with_opts(&enc, &DecodeOpts::default()).unwrap_err(), Error::ArrayTooLong);
        assert!(decode_with_opts(&enc, &DecodeOpts::permissive()).is_ok());
    }
}
