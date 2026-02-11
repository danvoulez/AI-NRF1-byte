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

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Runtime error with structured code + human message.
#[derive(Debug)]
pub struct PipelineError {
    pub code: ErrorCode,
    pub message: String,
    /// Optional inner error for debugging.
    pub source: Option<anyhow::Error>,
}

impl PipelineError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(mut self, err: anyhow::Error) -> Self {
        self.source = Some(err);
        self
    }

    /// JSON body for HTTP error responses.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "error": {
                "code": self.code.code(),
                "status": self.code.http_status(),
                "message": self.message,
            }
        })
    }
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.code(), self.message)
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
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
        assert_eq!(j["error"]["code"], "Err.Policy.Deny");
        assert_eq!(j["error"]["status"], 422);
        assert_eq!(j["error"]["message"], "risk score below threshold");
    }

    #[test]
    fn display_format() {
        let err = PipelineError::new(ErrorCode::HopBadChain, "prev mismatch at hop 3");
        assert_eq!(
            format!("{err}"),
            "[Err.Hop.BadChain] prev mismatch at hop 3"
        );
    }
}
