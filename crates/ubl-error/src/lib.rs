//! `ubl-error` — Central error station for the UBL ecosystem.
//!
//! Every error type in the codebase converts to `UblError` via `From` impls.
//! This gives every error a canonical shape that LLMs can parse and act on:
//!
//! ```json
//! {
//!   "error": {
//!     "code": "Err.NRF.InvalidMagic",
//!     "message": "expected 'nrf1' magic header, got [0x00, 0x00, 0x00, 0x00]",
//!     "hint": "Ensure the buffer starts with the 4-byte NRF magic: 0x6e726631",
//!     "status": 400
//!   }
//! }
//! ```
//!
//! Existing crates keep their own error enums for type safety.
//! This crate sits at the TOP of the dependency graph and depends on them.

use serde::Serialize;

// ---------------------------------------------------------------------------
// UblError — the ONE canonical error shape
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct UblError {
    pub code: String,
    pub message: String,
    pub hint: String,
    pub status: u16,
}

impl UblError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, hint: impl Into<String>, status: u16) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint: hint.into(),
            status,
        }
    }

    /// JSON body for HTTP error responses.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ok": false,
            "error": {
                "code": self.code,
                "message": self.message,
                "hint": self.hint,
                "status": self.status,
            }
        })
    }

    /// Single-line display for logs and CLI.
    pub fn one_line(&self) -> String {
        format!("[{}] {} → {}", self.code, self.message, self.hint)
    }

    /// Internal server error fallback.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            "Err.Internal",
            message,
            "This is an internal error. Check server logs for details.",
            500,
        )
    }
}

impl std::fmt::Display for UblError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {} → {}", self.code, self.message, self.hint)
    }
}

impl std::error::Error for UblError {}

// =========================================================================
// From impls — each crate's errors map to UblError with code + hint
// =========================================================================

// ---------------------------------------------------------------------------
// nrf-core::Error → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "nrf")]
impl From<nrf_core::Error> for UblError {
    fn from(e: nrf_core::Error) -> Self {
        use nrf_core::Error::*;
        let (code, hint) = match &e {
            InvalidMagic => (
                "Err.NRF.InvalidMagic",
                "Ensure the buffer starts with the 4-byte NRF magic: 0x6e726631 ('nrf1'). This is the first 4 bytes of any valid NRF-encoded value.",
            ),
            InvalidTypeTag(_tag) => (
                "Err.NRF.InvalidTypeTag",
                "Valid type tags are: 0x00 (Null), 0x01 (False), 0x02 (True), 0x03 (Int64), 0x04 (String), 0x05 (Bytes), 0x06 (Array), 0x07 (Map). Check the byte at the current decode position.",
            ),
            NonMinimalVarint => (
                "Err.NRF.NonMinimalVarint",
                "Varints must use the minimum number of bytes. Re-encode lengths using minimal varint encoding (no leading zero bytes).",
            ),
            UnexpectedEOF => (
                "Err.NRF.UnexpectedEOF",
                "The buffer ended before the value was fully decoded. Check that the buffer length matches the encoded size, or that the stream wasn't truncated.",
            ),
            InvalidUTF8 => (
                "Err.NRF.InvalidUTF8",
                "String values must be valid UTF-8. Check for invalid byte sequences. Use a UTF-8 validator before encoding.",
            ),
            NotNFC => (
                "Err.NRF.NotNFC",
                "Strings must be NFC-normalized (Unicode Canonical Decomposition followed by Canonical Composition). Apply unicode_normalization::UnicodeNormalization::nfc() before encoding.",
            ),
            BOMPresent => (
                "Err.NRF.BOMPresent",
                "Strings must not contain the Unicode BOM (U+FEFF). Strip the BOM character before encoding. It is forbidden by the ρ normalization rules.",
            ),
            NonStringKey => (
                "Err.NRF.NonStringKey",
                "Map keys must be strings (type tag 0x04). Other types are not allowed as map keys in NRF.",
            ),
            UnsortedKeys => (
                "Err.NRF.UnsortedKeys",
                "Map keys must be sorted lexicographically by their UTF-8 bytes. Use a BTreeMap to guarantee sorted key order.",
            ),
            DuplicateKey => (
                "Err.NRF.DuplicateKey",
                "Map keys must be unique. Remove the duplicate key. BTreeMap naturally deduplicates.",
            ),
            TrailingData => (
                "Err.NRF.TrailingData",
                "Extra bytes found after the encoded value. The buffer should contain exactly one NRF value. Check for concatenated or corrupted data.",
            ),
            DepthExceeded => (
                "Err.NRF.DepthExceeded",
                "Nesting depth exceeds the maximum allowed. Flatten the data structure or increase the depth limit.",
            ),
            SizeExceeded => (
                "Err.NRF.SizeExceeded",
                "Encoded size exceeds the maximum allowed. Reduce the payload size or increase the size limit.",
            ),
            Io(_) => (
                "Err.NRF.Io",
                "An I/O error occurred during encoding or decoding. Check file permissions, disk space, or network connectivity.",
            ),
            HexOddLength => (
                "Err.NRF.HexOddLength",
                "Hex strings must have an even number of characters (each byte = 2 hex chars). Check for a missing or extra character.",
            ),
            HexUppercase => (
                "Err.NRF.HexUppercase",
                "Hex strings must use lowercase characters (a-f, not A-F). Convert to lowercase before passing.",
            ),
            HexInvalidChar => (
                "Err.NRF.HexInvalidChar",
                "Hex strings must only contain characters 0-9 and a-f. Remove any non-hex characters.",
            ),
            NotASCII => (
                "Err.NRF.NotASCII",
                "This field requires ASCII-only characters (0x00-0x7F). DID and KID fields must be ASCII. Remove non-ASCII characters.",
            ),
            Float => (
                "Err.NRF.Float",
                "Floats are forbidden in NRF. Use Int64 instead. For decimals, multiply by the appropriate power of 10 (e.g., cents instead of dollars).",
            ),
        };
        UblError::new(code, format!("{e}"), hint, 400)
    }
}

// ---------------------------------------------------------------------------
// nrf-core::rho::RhoError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "nrf")]
impl From<nrf_core::rho::RhoError> for UblError {
    fn from(e: nrf_core::rho::RhoError) -> Self {
        use nrf_core::rho::RhoError::*;
        let (code, hint) = match &e {
            InvalidUTF8 => (
                "Err.Rho.InvalidUTF8",
                "The string contains a Unicode BOM (U+FEFF) which is forbidden by ρ rule 1b. Strip the BOM before passing the value.",
            ),
            InvalidDecimal(_) => (
                "Err.Rho.InvalidDecimal",
                "Decimal strings must match: no exponent, no leading zeros, no superfluous .0. Examples: '42', '-3.14', '1.5'. Invalid: '01.5', '1e2', '1.0' (use '1').",
            ),
            InvalidTimestamp(_) => (
                "Err.Rho.InvalidTimestamp",
                "Timestamps must be RFC-3339 UTC with 'Z' suffix and minimal fractional seconds. Example: '2024-01-15T10:30:00Z'. Invalid: '2024-01-15 10:30:00', '2024-01-15T10:30:00.000Z' (use no fraction if .000).",
            ),
        };
        UblError::new(code, format!("{e}"), hint, 400)
    }
}

// ---------------------------------------------------------------------------
// ubl_json_view::JsonViewError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "json_view")]
impl From<ubl_json_view::JsonViewError> for UblError {
    fn from(e: ubl_json_view::JsonViewError) -> Self {
        use ubl_json_view::JsonViewError::*;
        let (code, hint) = match &e {
            Float => (
                "Err.JsonView.Float",
                "Floats are forbidden in NRF. Use integers (Int64). For monetary values use cents, for percentages use basis points (1% = 100).",
            ),
            InvalidUTF8 => (
                "Err.JsonView.InvalidUTF8",
                "String is not valid UTF-8. Ensure all strings are properly UTF-8 encoded before converting to NRF.",
            ),
            NotNFC => (
                "Err.JsonView.NotNFC",
                "String is not NFC-normalized. Apply NFC normalization (e.g., unicode_normalization::nfc()) before converting.",
            ),
            BOMPresent => (
                "Err.JsonView.BOMPresent",
                "String contains Unicode BOM (U+FEFF). Strip the BOM character. It is forbidden by ρ normalization rules.",
            ),
            OddHex => (
                "Err.JsonView.OddHex",
                "Hex string has odd length. Each byte requires exactly 2 hex characters. Add a leading '0' if needed.",
            ),
            BadHex => (
                "Err.JsonView.BadHex",
                "Invalid hex character found. Hex strings must only contain 0-9 and a-f (lowercase). Check for typos or uppercase letters.",
            ),
            BadBase64(_) => (
                "Err.JsonView.BadBase64",
                "Invalid base64 encoding. Use standard base64 (RFC 4648). Check for missing padding '=' characters or invalid characters.",
            ),
            BadPrefix => (
                "Err.JsonView.BadPrefix",
                "Unknown bytes prefix. Valid prefixes: 'b3:' (BLAKE3 hash, 32 bytes), 'b64:' (base64, 16 or 64 bytes), or {\"$bytes\": \"<base64>\"} for other sizes.",
            ),
            NotASCII => (
                "Err.JsonView.NotASCII",
                "DID and KID fields must be ASCII-only (0x00-0x7F). Remove any non-ASCII characters from identifier fields.",
            ),
            InvalidDecimal(_) => (
                "Err.JsonView.InvalidDecimal",
                "Decimal string is not canonical. Rules: no exponent, no leading zeros, no trailing '.0'. Use integers when possible.",
            ),
            IntegerOverflow => (
                "Err.JsonView.IntegerOverflow",
                "Number exceeds Int64 range (-2^63 to 2^63-1). Use a smaller number or represent as a string.",
            ),
            NonMinimalVarint => (
                "Err.JsonView.NonMinimalVarint",
                "Varint encoding is not minimal. Re-encode using the minimum number of bytes for the value.",
            ),
            NrfDecode(_) => (
                "Err.JsonView.NrfDecode",
                "Failed to decode NRF bytes. The binary data may be corrupted or not valid NRF. Re-encode from the source value.",
            ),
        };
        UblError::new(code, format!("{e}"), hint, 400)
    }
}

// ---------------------------------------------------------------------------
// ubl_capsule::receipt::HopError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "capsule")]
impl From<ubl_capsule::receipt::HopError> for UblError {
    fn from(e: ubl_capsule::receipt::HopError) -> Self {
        use ubl_capsule::receipt::HopError::*;
        let (code, hint, status) = match &e {
            BadChain(_i) => (
                "Err.Hop.BadChain",
                "Receipt chain is broken: receipt[N].prev does not match the ID of receipt[N-1]. Verify the chain was built sequentially with each receipt's prev pointing to the previous receipt's ID.",
                422,
            ),
            BadSignature(_i) => (
                "Err.Hop.BadSignature",
                "Receipt signature verification failed. The signing key may not match the node's public key, or the receipt payload was modified after signing.",
                403,
            ),
            BadDomain => (
                "Err.Hop.BadDomain",
                "Receipt domain must be 'ubl-receipt/1.0'. Check the 'domain' field in the receipt payload.",
                400,
            ),
            NotASCII => (
                "Err.Hop.NotASCII",
                "Receipt node identifier must be ASCII-only. DIDs and node names must not contain non-ASCII characters.",
                400,
            ),
            Fork(_i) => (
                "Err.Hop.Fork",
                "Duplicate 'prev' value detected — two receipts claim the same predecessor. This indicates a fork in the receipt chain, which is forbidden.",
                422,
            ),
        };
        UblError::new(code, format!("{e}"), hint, status)
    }
}

// ---------------------------------------------------------------------------
// ubl_capsule::seal::SealError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "capsule")]
impl From<ubl_capsule::seal::SealError> for UblError {
    fn from(e: ubl_capsule::seal::SealError) -> Self {
        use ubl_capsule::seal::SealError::*;
        let (code, hint, status) = match &e {
            BadDomain => (
                "Err.Seal.BadDomain",
                "Capsule domain must be 'ubl-capsule/1.0'. Check the 'domain' field in the capsule.",
                400,
            ),
            BadScope => (
                "Err.Seal.BadScope",
                "Seal scope must be 'capsule'. Check the 'scope' field in the seal.",
                400,
            ),
            BadAudience => (
                "Err.Seal.BadAudience",
                "Seal audience (aud) does not match the capsule header destination (hdr.dst). Ensure the seal was created for this specific recipient.",
                403,
            ),
            BadSignature => (
                "Err.Seal.BadSignature",
                "Seal signature verification failed. The signing key may not match the author's public key, or the capsule was modified after sealing.",
                403,
            ),
            IdMismatch => (
                "Err.Seal.IdMismatch",
                "Capsule ID does not match the computed ID (BLAKE3 hash of the canonical payload). The capsule may have been tampered with. Recompute the ID from the payload.",
                422,
            ),
            Expired { exp: _, now: _ } => (
                "Err.Seal.Expired",
                "Capsule has expired. The expiration timestamp is in the past. Create a new capsule with a future expiration, or check clock synchronization.",
                410,
            ),
        };
        UblError::new(code, format!("{e}"), hint, status)
    }
}

// ---------------------------------------------------------------------------
// runtime::RuntimeError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "rt")]
impl From<runtime::RuntimeError> for UblError {
    fn from(e: runtime::RuntimeError) -> Self {
        use runtime::RuntimeError::*;
        let (code, hint) = match &e {
            EmptyBinarySha => (
                "Err.Runtime.EmptyBinarySha",
                "Set the BINARY_SHA256 environment variable to the SHA-256 hash of the running binary. In dev mode, any non-empty string works.",
            ),
            EmptyName => (
                "Err.Runtime.EmptyName",
                "RuntimeInfo.name must not be empty. Set it to the service name (e.g., 'registry', 'runner').",
            ),
            EmptyVersion => (
                "Err.Runtime.EmptyVersion",
                "RuntimeInfo.version must not be empty. Set it to the service version (e.g., '1.0.0').",
            ),
            RhoFailed(_) => (
                "Err.Runtime.RhoFailed",
                "ρ normalization failed on RuntimeInfo. Check that all string fields are valid UTF-8 and NFC-normalized, and no field contains a BOM.",
            ),
            NotCanonical(_) => (
                "Err.Runtime.NotCanonical",
                "RuntimeInfo is not in ρ-canonical form. Apply rho::normalize() before using. Common causes: non-NFC strings, null values in maps.",
            ),
            AttestationFailed(_) => (
                "Err.Runtime.AttestationFailed",
                "Runtime attestation failed. Check that the binary hash matches, TEE is available (if required), and all certs are valid.",
            ),
            EmptyCert(_) => (
                "Err.Runtime.EmptyCert",
                "A certificate in the certs array is empty. Remove empty entries or provide valid certificate bytes.",
            ),
        };
        UblError::new(code, format!("{e}"), hint, 500)
    }
}

// ---------------------------------------------------------------------------
// ubl_storage::ledger::LedgerError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "storage")]
impl From<ubl_storage::ledger::LedgerError> for UblError {
    fn from(e: ubl_storage::ledger::LedgerError) -> Self {
        use ubl_storage::ledger::LedgerError::*;
        let (code, hint) = match &e {
            Io(_) => (
                "Err.Ledger.Io",
                "Ledger I/O error. Check that LEDGER_DIR exists and is writable, disk is not full, and file permissions are correct.",
            ),
            Serialization(_) => (
                "Err.Ledger.Serialization",
                "Failed to serialize/deserialize a ledger entry. Check that the payload contains only NRF-compatible types (no floats, valid UTF-8 strings).",
            ),
        };
        UblError::new(code, format!("{e}"), hint, 500)
    }
}

// ---------------------------------------------------------------------------
// ubl_replay::ReplayError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "replay")]
impl From<ubl_replay::ReplayError> for UblError {
    fn from(e: ubl_replay::ReplayError) -> Self {
        use ubl_replay::ReplayError::*;
        let (code, hint, status) = match &e {
            Replayed => (
                "Err.Replay.Replayed",
                "This (src, nonce) pair was already seen. Generate a new unique nonce (16 random bytes) for each request.",
                409,
            ),
            Expired => (
                "Err.Replay.Expired",
                "The request has expired (exp <= now). Set hdr.exp to a future timestamp. Check clock synchronization between client and server.",
                410,
            ),
            BadCapacity => (
                "Err.Replay.BadCapacity",
                "Replay cache capacity must be > 0. Set a positive integer for the cache size.",
                500,
            ),
        };
        UblError::new(code, format!("{e}"), hint, status)
    }
}

// ---------------------------------------------------------------------------
// ubl_auth::AuthError → UblError
// ---------------------------------------------------------------------------
#[cfg(feature = "auth")]
impl From<ubl_auth::AuthError> for UblError {
    fn from(e: ubl_auth::AuthError) -> Self {
        use ubl_auth::AuthError::*;
        let (code, hint, status) = match &e {
            Unauthorized => (
                "Err.Auth.Unauthorized",
                "Authentication required. Provide X-DID, X-Signature (base64 Ed25519 over 'METHOD|PATH'), and X-Pubkey headers.",
                401,
            ),
            Forbidden => (
                "Err.Auth.Forbidden",
                "You do not have the required role for this action. Check your role assignments for this tenant.",
                403,
            ),
            BadHeader(_field) => (
                "Err.Auth.BadHeader",
                "A required authentication header is missing or malformed. Required headers: X-DID, X-Signature, X-Pubkey, X-Method, X-Path.",
                400,
            ),
            InvalidSignature => (
                "Err.Auth.InvalidSignature",
                "Signature verification failed. Ensure: (1) X-Signature is base64-encoded Ed25519 signature, (2) signed message is 'METHOD|PATH', (3) X-Pubkey matches the signing key.",
                401,
            ),
        };
        UblError::new(code, format!("{e}"), hint, status)
    }
}

// =========================================================================
// Convenience: middleware / API error constructors
// =========================================================================

impl UblError {
    pub fn missing_header(header: &str, hint: &str) -> Self {
        Self::new(
            "Err.Request.MissingHeader",
            format!("missing required header: {header}"),
            hint,
            400,
        )
    }

    pub fn invalid_api_key(product: &str) -> Self {
        Self::new(
            "Err.Auth.InvalidApiKey",
            format!("invalid or missing API key for product '{product}'"),
            "Set the X-API-Key header to the key registered for this product. Keys are configured via the API_KEYS environment variable on the server.",
            401,
        )
    }

    pub fn rate_limited(product: &str) -> Self {
        Self::new(
            "Err.RateLimit.Exceeded",
            format!("rate limit exceeded for product '{product}'"),
            "Wait before retrying. The server enforces per-product rate limits configured via RATE_LIMIT_RPM. Use exponential backoff.",
            429,
        )
    }

    pub fn bad_request(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self::new("Err.Request.BadRequest", message, hint, 400)
    }

    pub fn not_found(entity: &str, id: &str) -> Self {
        Self::new(
            "Err.NotFound",
            format!("{entity} '{id}' not found"),
            format!("Check that the {entity} ID is correct and that it exists in this tenant's data."),
            404,
        )
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ubl_error_json_shape() {
        let e = UblError::new("Err.Test.Example", "something broke", "fix it like this", 400);
        let j = e.to_json();
        assert_eq!(j["ok"], false);
        assert_eq!(j["error"]["code"], "Err.Test.Example");
        assert_eq!(j["error"]["message"], "something broke");
        assert_eq!(j["error"]["hint"], "fix it like this");
        assert_eq!(j["error"]["status"], 400);
    }

    #[test]
    fn test_one_line_format() {
        let e = UblError::new("Err.NRF.Float", "floats forbidden", "use Int64", 400);
        assert_eq!(e.one_line(), "[Err.NRF.Float] floats forbidden → use Int64");
    }

    #[test]
    fn test_display() {
        let e = UblError::internal("oops");
        assert!(format!("{e}").contains("Err.Internal"));
        assert!(format!("{e}").contains("oops"));
    }

    #[cfg(feature = "nrf")]
    #[test]
    fn test_nrf_error_conversion() {
        let nrf_err = nrf_core::Error::InvalidMagic;
        let ubl: UblError = nrf_err.into();
        assert_eq!(ubl.code, "Err.NRF.InvalidMagic");
        assert!(ubl.hint.contains("nrf1"));
        assert_eq!(ubl.status, 400);
    }

    #[cfg(feature = "nrf")]
    #[test]
    fn test_rho_error_conversion() {
        let rho_err = nrf_core::rho::RhoError::InvalidTimestamp("bad".into());
        let ubl: UblError = rho_err.into();
        assert_eq!(ubl.code, "Err.Rho.InvalidTimestamp");
        assert!(ubl.hint.contains("RFC-3339"));
    }

    #[cfg(feature = "json_view")]
    #[test]
    fn test_json_view_error_conversion() {
        let jv_err = ubl_json_view::JsonViewError::Float;
        let ubl: UblError = jv_err.into();
        assert_eq!(ubl.code, "Err.JsonView.Float");
        assert!(ubl.hint.contains("Int64"));
    }

    #[test]
    fn test_convenience_constructors() {
        let e = UblError::missing_header("X-Tenant", "Add X-Tenant header");
        assert_eq!(e.code, "Err.Request.MissingHeader");
        assert_eq!(e.status, 400);

        let e = UblError::invalid_api_key("tdln");
        assert_eq!(e.code, "Err.Auth.InvalidApiKey");
        assert_eq!(e.status, 401);

        let e = UblError::rate_limited("tdln");
        assert_eq!(e.code, "Err.RateLimit.Exceeded");
        assert_eq!(e.status, 429);

        let e = UblError::not_found("receipt", "b3:abc123");
        assert_eq!(e.code, "Err.NotFound");
        assert_eq!(e.status, 404);
    }
}
