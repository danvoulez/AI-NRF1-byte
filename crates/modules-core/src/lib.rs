//! Module contract layer for the AI-NRF1 capability system.
//!
//! Defines the universal envelope (`CapInput`/`CapOutput`), the `Capability` trait,
//! `Verdict`, `Effect`, `AssetResolver`, and supporting types.
//!
//! See `docs/MODULES-DESIGN.md` sections 0–3 for rationale.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Fundamental types
// ---------------------------------------------------------------------------

/// Content-addressed identifier (BLAKE3, 32 bytes).
pub type Cid = [u8; 32];

/// Execution metadata injected by the runtime into every capability call.
#[derive(Clone, Debug)]
pub struct ExecutionMeta {
    pub run_id: String,
    pub tenant: Option<String>,
    pub trace_id: Option<String>,
    pub ts_nanos: i64,
}

// ---------------------------------------------------------------------------
// Assets (content-addressed IO — design doc §1, §4)
// ---------------------------------------------------------------------------

/// A resolved asset blob.
#[derive(Clone, Debug)]
pub struct Asset {
    pub cid: Cid,
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// Resolve assets by CID. Implemented by the runtime (MemoryResolver, S3, FS, …).
pub trait AssetResolver: Send + Sync {
    fn get(&self, cid: &Cid) -> anyhow::Result<Asset>;
    fn box_clone(&self) -> Box<dyn AssetResolver>;
}

impl Clone for Box<dyn AssetResolver> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

impl std::fmt::Debug for dyn AssetResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AssetResolver(..)")
    }
}

// ---------------------------------------------------------------------------
// Artifacts & Effects (design doc §3 — "separar resultado de efeito")
// ---------------------------------------------------------------------------

/// A derived output blob (HTML, SVG, PDF, JSON payload, …).
#[derive(Clone, Debug)]
pub struct Artifact {
    pub cid: Option<Cid>,
    pub mime: String,
    pub bytes: Vec<u8>,
    pub name: Option<String>,
}

/// Declarative side-effects returned by a capability.
/// The runtime's `EffectExecutor` decides when/how to execute them.
#[derive(Clone, Debug)]
pub enum Effect {
    // --- enrich / general ---
    Webhook {
        url: String,
        body: Vec<u8>,
        content_type: String,
        hmac_key_env: Option<String>,
    },
    WriteStorage {
        path: String,
        bytes: Vec<u8>,
        mime: String,
    },

    // --- permit (consent K-of-N) ---
    QueueConsentTicket {
        ticket_id: String,
        expires_at: i64,
        required_roles: Vec<String>,
        k: u8,
        n: u8,
    },
    CloseConsentTicket {
        ticket_id: String,
        /// "ALLOW" | "DENY" | "EXPIRED"
        outcome: String,
    },

    // --- transport (SIRP / relay) ---
    AppendReceipt {
        /// NRF-encoded receipt payload (without sig).
        payload_nrf: Vec<u8>,
        /// Binding name for the signing key (resolved by executor).
        signer_binding: String,
    },
    RelayOut {
        /// Transport kind (e.g. "http").
        to: String,
        /// Binding name for the destination URL.
        url_binding: String,
        /// Capsule body to relay (JSON).
        body: Vec<u8>,
    },

    // --- llm (assist) ---
    InvokeLlm {
        /// Binding name for the model provider (resolved by executor).
        model_binding: String,
        /// Rendered prompt text.
        prompt: String,
        /// Max tokens for the response.
        max_tokens: u32,
        /// Cache key: hash of (prompt_cid, inputs).
        cache_key: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// Verdict (design doc §9B — cap-policy output)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Allow,
    Deny,
    Require,
}

// ---------------------------------------------------------------------------
// CapInput / CapOutput — the universal envelope (design doc §2)
// ---------------------------------------------------------------------------

/// Universal input envelope for every capability.
#[derive(Clone, Debug)]
pub struct CapInput {
    /// Canonical environment (ai-json-nrf1 semantics).
    pub env: nrf1::Value,
    /// Config fragment from the product manifest for this step.
    pub config: serde_json::Value,
    /// Resolved assets (packs, templates, …) by CID.
    pub assets: Box<dyn AssetResolver>,
    /// CIDs of receipts from previous pipeline steps.
    pub prev_receipts: Vec<Cid>,
    /// Runtime-injected execution metadata.
    pub meta: ExecutionMeta,
}

/// Universal output envelope from every capability.
#[derive(Clone, Debug, Default)]
pub struct CapOutput {
    /// Optionally updated environment for the next step.
    pub new_env: Option<nrf1::Value>,
    /// Decision (only set by policy-family capabilities).
    pub verdict: Option<Verdict>,
    /// Derived artifacts (HTML, SVG, …).
    pub artifacts: Vec<Artifact>,
    /// Declarative side-effects for the runtime to execute.
    pub effects: Vec<Effect>,
    /// Observability metrics (key, value).
    pub metrics: Vec<(String, i64)>,
}

// ---------------------------------------------------------------------------
// Capability trait (design doc §2, §6 — versionamento)
// ---------------------------------------------------------------------------

/// The single trait every module implements.
///
/// - `kind()` / `api_version()` are methods (not associated consts) so the
///   trait is object-safe and works with `dyn Capability`.
/// - `execute()` is **pure**: no IO, no network, no DB.
///   Side-effects are returned as `Effect` variants.
pub trait Capability: Send + Sync {
    /// Module identity, e.g. `"cap-intake"`, `"cap-policy"`.
    fn kind(&self) -> &'static str;

    /// Semantic version of the capability API, e.g. `"1.0"`.
    fn api_version(&self) -> &'static str;

    /// Validate the config fragment from the product manifest.
    fn validate_config(&self, config: &serde_json::Value) -> anyhow::Result<()>;

    /// Pure, deterministic execution. No IO.
    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput>;
}
