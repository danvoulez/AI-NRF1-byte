//! Formal error types for the module runtime.
//!
//! Maps internal errors to the `Err.*` taxonomy defined in the design doc,
//! and provides HTTP status code mapping for API surfaces.

use std::fmt;

/// Structured error code following the `Err.<Category>.<Detail>` convention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    // --- Canon / Parse ---
    CanonNotAscii,
    CanonInvalidNrf,
    CanonParseFailed,

    // --- Header ---
    HdrExpired,
    HdrMissingField,

    // --- Seal / Auth ---
    SealBadSignature,
    SealMissing,
    AuthUnauthorized,
    AuthForbidden,

    // --- Policy ---
    PolicyDeny,
    PolicyRequire,

    // --- Hop / Receipt ---
    HopBadChain,
    HopBadSignature,
    HopMissing,

    // --- Replay / Idempotency ---
    Replay,
    IdempotencyConflict,

    // --- Permit / Consent ---
    PermitExpired,
    PermitInvalidRole,
    PermitQuorumNotMet,

    // --- IO / External ---
    IoWebhookFailed,
    IoRelayFailed,
    IoStorageFailed,
    IoLlmFailed,

    // --- Config ---
    ConfigInvalid,
    ConfigCapNotFound,

    // --- Internal ---
    Internal,
}

impl ErrorCode {
    /// Canonical string representation: `Err.<Category>.<Detail>`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::CanonNotAscii => "Err.Canon.NotASCII",
            Self::CanonInvalidNrf => "Err.Canon.InvalidNRF",
            Self::CanonParseFailed => "Err.Canon.ParseFailed",
            Self::HdrExpired => "Err.Hdr.Expired",
            Self::HdrMissingField => "Err.Hdr.MissingField",
            Self::SealBadSignature => "Err.Seal.BadSignature",
            Self::SealMissing => "Err.Seal.Missing",
            Self::AuthUnauthorized => "Err.Auth.Unauthorized",
            Self::AuthForbidden => "Err.Auth.Forbidden",
            Self::PolicyDeny => "Err.Policy.Deny",
            Self::PolicyRequire => "Err.Policy.Require",
            Self::HopBadChain => "Err.Hop.BadChain",
            Self::HopBadSignature => "Err.Hop.BadSignature",
            Self::HopMissing => "Err.Hop.Missing",
            Self::Replay => "Err.Replay",
            Self::IdempotencyConflict => "Err.Idempotency.Conflict",
            Self::PermitExpired => "Err.Permit.Expired",
            Self::PermitInvalidRole => "Err.Permit.InvalidRole",
            Self::PermitQuorumNotMet => "Err.Permit.QuorumNotMet",
            Self::IoWebhookFailed => "Err.IO.WebhookFailed",
            Self::IoRelayFailed => "Err.IO.RelayFailed",
            Self::IoStorageFailed => "Err.IO.StorageFailed",
            Self::IoLlmFailed => "Err.IO.LlmFailed",
            Self::ConfigInvalid => "Err.Config.Invalid",
            Self::ConfigCapNotFound => "Err.Config.CapNotFound",
            Self::Internal => "Err.Internal",
        }
    }

    /// Recommended HTTP status code for API responses.
    pub fn http_status(&self) -> u16 {
        match self {
            // 400 — canon/parse
            Self::CanonNotAscii
            | Self::CanonInvalidNrf
            | Self::CanonParseFailed
            | Self::HdrMissingField
            | Self::ConfigInvalid => 400,

            // 401 — auth
            Self::AuthUnauthorized | Self::SealMissing => 401,

            // 403 — forbidden
            Self::AuthForbidden | Self::SealBadSignature => 403,

            // 404 — not found
            Self::ConfigCapNotFound => 404,

            // 409 — conflict (replay/idempotency)
            Self::Replay | Self::IdempotencyConflict => 409,

            // 410 — gone (expired)
            Self::HdrExpired | Self::PermitExpired => 410,

            // 422 — unprocessable (policy DENY/REQUIRE, bad chain, quorum)
            Self::PolicyDeny
            | Self::PolicyRequire
            | Self::HopBadChain
            | Self::HopBadSignature
            | Self::HopMissing
            | Self::PermitInvalidRole
            | Self::PermitQuorumNotMet => 422,

            // 500 — internal
            Self::Internal => 500,

            // 502 — bad gateway (external IO)
            Self::IoWebhookFailed
            | Self::IoRelayFailed
            | Self::IoStorageFailed
            | Self::IoLlmFailed => 502,
        }
    }
}

impl ErrorCode {
    /// Actionable hint for LLMs — what to do when this error occurs.
    pub fn hint(&self) -> &'static str {
        match self {
            Self::CanonNotAscii => "Ensure all identifier fields (DID, KID) contain only ASCII characters (0x00-0x7F).",
            Self::CanonInvalidNrf => "The NRF binary payload is malformed. Re-encode from the source Value using nrf_core::encode().",
            Self::CanonParseFailed => "Input could not be parsed. Check that the payload is valid JSON or NRF binary.",
            Self::HdrExpired => "The request header 'exp' timestamp is in the past. Set hdr.exp to a future UTC timestamp and retry.",
            Self::HdrMissingField => "A required header field is missing. Required: src, dst, exp, nonce. Check the capsule header.",
            Self::SealBadSignature => "Seal signature verification failed. Ensure the signing key matches the author's public key and the payload was not modified after sealing.",
            Self::SealMissing => "No seal found on the capsule. Sign the capsule with the author's Ed25519 key before submitting.",
            Self::AuthUnauthorized => "Authentication required. Provide valid credentials (X-DID, X-Signature, X-Pubkey headers).",
            Self::AuthForbidden => "You do not have the required role for this action. Check role assignments for this tenant.",
            Self::PolicyDeny => "A policy rule returned DENY. Check the policy evaluation details in the receipt chain to see which rule failed.",
            Self::PolicyRequire => "A policy rule returned REQUIRE (human approval needed). Create a permit ticket and wait for K-of-N approvals.",
            Self::HopBadChain => "Receipt chain is broken: receipt[N].prev does not match receipt[N-1].id. Verify chain was built sequentially.",
            Self::HopBadSignature => "A receipt signature is invalid. The signing node's key may not match, or the receipt was tampered with.",
            Self::HopMissing => "Expected receipt hop is missing from the chain. Ensure all pipeline steps produced a receipt.",
            Self::Replay => "This (src, nonce) pair was already seen. Generate a new unique nonce for each request.",
            Self::IdempotencyConflict => "An idempotency key conflict was detected. The same key was used with different parameters. Use a new idempotency key.",
            Self::PermitExpired => "The permit ticket has expired. Create a new ticket with a future expiration.",
            Self::PermitInvalidRole => "The approver's role is not in the permit's required_roles list. Use an approver with the correct role.",
            Self::PermitQuorumNotMet => "Not enough approvals yet. The permit requires K-of-N approvals. Wait for more approvers or lower the quorum.",
            Self::IoWebhookFailed => "Webhook delivery failed. Check that the webhook URL is reachable and returns 2xx. The runtime will retry with exponential backoff.",
            Self::IoRelayFailed => "Relay delivery failed. Check the relay endpoint URL and network connectivity.",
            Self::IoStorageFailed => "Storage write failed. Check that LEDGER_DIR / storage path exists and is writable, and disk is not full.",
            Self::IoLlmFailed => "LLM provider call failed. Check API key, model availability, and rate limits. The cached provider will retry.",
            Self::ConfigInvalid => "Pipeline or step configuration is invalid. Check that all required config fields are present and correctly typed.",
            Self::ConfigCapNotFound => "Capability not found in the registry. Check that the 'use' field matches a registered capability kind and the version is compatible.",
            Self::Internal => "An internal error occurred. Check server logs for details.",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Runtime error with structured code + human message + actionable hint.
#[derive(Debug)]
pub struct PipelineError {
    pub code: ErrorCode,
    pub message: String,
    /// Actionable suggestion for fixing the error (LLM-friendly).
    pub hint: String,
    /// Optional inner error for debugging.
    pub source: Option<anyhow::Error>,
}

impl PipelineError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        let c = &code;
        let hint = c.hint().to_string();
        Self {
            code,
            message: message.into(),
            hint,
            source: None,
        }
    }

    /// Override the default hint with a more specific one.
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = hint.into();
        self
    }

    pub fn with_source(mut self, err: anyhow::Error) -> Self {
        self.source = Some(err);
        self
    }

    /// JSON body for HTTP error responses.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ok": false,
            "error": {
                "code": self.code.code(),
                "status": self.code.http_status(),
                "message": self.message,
                "hint": self.hint,
            }
        })
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} → {}", self.code.code(), self.message, self.hint)
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_format() {
        assert_eq!(ErrorCode::HdrExpired.code(), "Err.Hdr.Expired");
        assert_eq!(ErrorCode::HopBadChain.code(), "Err.Hop.BadChain");
        assert_eq!(ErrorCode::Replay.code(), "Err.Replay");
    }

    #[test]
    fn http_status_mapping() {
        assert_eq!(ErrorCode::CanonNotAscii.http_status(), 400);
        assert_eq!(ErrorCode::AuthUnauthorized.http_status(), 401);
        assert_eq!(ErrorCode::SealBadSignature.http_status(), 403);
        assert_eq!(ErrorCode::Replay.http_status(), 409);
        assert_eq!(ErrorCode::HdrExpired.http_status(), 410);
        assert_eq!(ErrorCode::PolicyDeny.http_status(), 422);
        assert_eq!(ErrorCode::Internal.http_status(), 500);
        assert_eq!(ErrorCode::IoWebhookFailed.http_status(), 502);
    }

    #[test]
    fn pipeline_error_json() {
        let err = PipelineError::new(ErrorCode::PolicyDeny, "risk score below threshold");
        let j = err.to_json();
        assert_eq!(j["ok"], false);
        assert_eq!(j["error"]["code"], "Err.Policy.Deny");
        assert_eq!(j["error"]["status"], 422);
        assert_eq!(j["error"]["message"], "risk score below threshold");
        assert!(j["error"]["hint"].as_str().unwrap().contains("DENY"));
    }

    #[test]
    fn pipeline_error_custom_hint() {
        let err = PipelineError::new(ErrorCode::ConfigInvalid, "missing 'executor' field")
            .with_hint("Add 'executor: wasmtime' to the cap-runtime config");
        let j = err.to_json();
        assert_eq!(j["error"]["hint"], "Add 'executor: wasmtime' to the cap-runtime config");
    }

    #[test]
    fn display_format() {
        let err = PipelineError::new(ErrorCode::HopBadChain, "prev mismatch at hop 3");
        let s = format!("{err}");
        assert!(s.starts_with("[Err.Hop.BadChain] prev mismatch at hop 3"));
        assert!(s.contains("→"));
    }

    #[test]
    fn all_error_codes_have_hints() {
        let codes = [
            ErrorCode::CanonNotAscii, ErrorCode::CanonInvalidNrf, ErrorCode::CanonParseFailed,
            ErrorCode::HdrExpired, ErrorCode::HdrMissingField,
            ErrorCode::SealBadSignature, ErrorCode::SealMissing,
            ErrorCode::AuthUnauthorized, ErrorCode::AuthForbidden,
            ErrorCode::PolicyDeny, ErrorCode::PolicyRequire,
            ErrorCode::HopBadChain, ErrorCode::HopBadSignature, ErrorCode::HopMissing,
            ErrorCode::Replay, ErrorCode::IdempotencyConflict,
            ErrorCode::PermitExpired, ErrorCode::PermitInvalidRole, ErrorCode::PermitQuorumNotMet,
            ErrorCode::IoWebhookFailed, ErrorCode::IoRelayFailed, ErrorCode::IoStorageFailed, ErrorCode::IoLlmFailed,
            ErrorCode::ConfigInvalid, ErrorCode::ConfigCapNotFound,
            ErrorCode::Internal,
        ];
        for code in &codes {
            let hint = code.hint();
            assert!(!hint.is_empty(), "{} has empty hint", code.code());
            assert!(hint.len() > 20, "{} hint too short: {}", code.code(), hint);
        }
    }
}
