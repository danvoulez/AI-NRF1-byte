//! EffectExecutor — dispatches declarative `Effect` variants to concrete adapters.
//! See design doc §3: "capabilities return effects, runtime executes them."
//!
//! The executor resolves bindings, enforces idempotency, and logs structured
//! events. It never leaks secret values into logs.

use async_trait::async_trait;
use modules_core::Effect;
use std::collections::HashSet;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// ExecCtx — context passed to every effect execution
// ---------------------------------------------------------------------------

/// Execution context for effect dispatch.
#[derive(Clone, Debug)]
pub struct ExecCtx {
    pub tenant: String,
    pub trace_id: String,
    pub io_bindings: serde_json::Value,
    pub now_nanos: i64,
    pub step_id: String,
    pub capsule_id_hex: String,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait EffectExecutor: Send + Sync {
    async fn execute(&self, effect: &Effect, ctx: &ExecCtx) -> anyhow::Result<()>;
}

// ---------------------------------------------------------------------------
// NoopExecutor (for tests)
// ---------------------------------------------------------------------------

pub struct NoopExecutor;

#[async_trait]
impl EffectExecutor for NoopExecutor {
    async fn execute(&self, _effect: &Effect, _ctx: &ExecCtx) -> anyhow::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// LoggingExecutor — logs effects as structured JSON, executes nothing.
// Good for dry-run / audit.
// ---------------------------------------------------------------------------

pub struct LoggingExecutor;

#[async_trait]
impl EffectExecutor for LoggingExecutor {
    async fn execute(&self, effect: &Effect, ctx: &ExecCtx) -> anyhow::Result<()> {
        tracing::info!(
            tenant = %ctx.tenant,
            trace_id = %ctx.trace_id,
            step_id = %ctx.step_id,
            capsule_id = %ctx.capsule_id_hex,
            effect_kind = effect_kind(effect),
            "effect.dispatch (dry-run)"
        );
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// IdempotentExecutor — wraps another executor, deduplicates by run_key.
// ---------------------------------------------------------------------------

pub struct IdempotentExecutor<E: EffectExecutor> {
    inner: E,
    seen: Mutex<HashSet<String>>,
}

impl<E: EffectExecutor> IdempotentExecutor<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            seen: Mutex::new(HashSet::new()),
        }
    }

    /// Compute idempotency key: blake3(capsule_id || step_id || effect_kind).
    fn run_key(effect: &Effect, ctx: &ExecCtx) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(ctx.capsule_id_hex.as_bytes());
        hasher.update(ctx.step_id.as_bytes());
        hasher.update(effect_kind(effect).as_bytes());
        // Include effect-specific discriminator
        match effect {
            Effect::Webhook { url, .. } => hasher.update(url.as_bytes()),
            Effect::WriteStorage { path, .. } => hasher.update(path.as_bytes()),
            Effect::QueueConsentTicket { ticket_id, .. } => hasher.update(ticket_id.as_bytes()),
            Effect::CloseConsentTicket { ticket_id, .. } => hasher.update(ticket_id.as_bytes()),
            Effect::AppendReceipt { signer_binding, .. } => {
                hasher.update(signer_binding.as_bytes())
            }
            Effect::RelayOut { url_binding, .. } => hasher.update(url_binding.as_bytes()),
            Effect::InvokeLlm { cache_key, .. } => {
                if let Some(ck) = cache_key {
                    hasher.update(ck.as_bytes())
                } else {
                    hasher.update(b"no-cache-key")
                }
            }
        };
        hex::encode(hasher.finalize().as_bytes())
    }
}

#[async_trait]
impl<E: EffectExecutor> EffectExecutor for IdempotentExecutor<E> {
    async fn execute(&self, effect: &Effect, ctx: &ExecCtx) -> anyhow::Result<()> {
        let key = Self::run_key(effect, ctx);
        {
            let mut seen = self.seen.lock().unwrap();
            if seen.contains(&key) {
                tracing::debug!(
                    idempotency_key = %key,
                    effect_kind = effect_kind(effect),
                    "effect.skipped (idempotent)"
                );
                return Ok(());
            }
            seen.insert(key.clone());
        }
        self.inner.execute(effect, ctx).await
    }
}

// ---------------------------------------------------------------------------
// DispatchExecutor — routes each Effect variant to its adapter.
// This is the "production" executor. Use DispatchBuilder to configure.
// ---------------------------------------------------------------------------

#[cfg(feature = "live")]
use crate::adapters::http::idempotency_key;
use crate::adapters::permit::{PermitStore, Ticket, TicketStatus};
use std::sync::Arc;

pub struct DispatchExecutor {
    pub storage_base: String,
    /// HTTP adapter for Webhook and RelayOut (None = log-only stub).
    #[cfg(feature = "live")]
    http: Option<Arc<crate::adapters::http::HttpClient>>,
    /// Receipt signer (None = NoopSigner).
    signer: Option<Arc<dyn crate::adapters::signer::ReceiptSigner>>,
    /// LLM provider (None = log-only stub).
    llm: Option<Arc<dyn crate::adapters::llm::LlmProvider>>,
    /// Permit store (None = raw JSON file fallback).
    permit_store: Option<Arc<PermitStore>>,
}

/// Builder for DispatchExecutor.
pub struct DispatchBuilder {
    storage_base: String,
    #[cfg(feature = "live")]
    http: Option<Arc<crate::adapters::http::HttpClient>>,
    signer: Option<Arc<dyn crate::adapters::signer::ReceiptSigner>>,
    llm: Option<Arc<dyn crate::adapters::llm::LlmProvider>>,
    permit_store: Option<Arc<PermitStore>>,
}

impl DispatchBuilder {
    pub fn new(storage_base: impl Into<String>) -> Self {
        Self {
            storage_base: storage_base.into(),
            #[cfg(feature = "live")]
            http: None,
            signer: None,
            llm: None,
            permit_store: None,
        }
    }

    #[cfg(feature = "live")]
    pub fn http(mut self, client: crate::adapters::http::HttpClient) -> Self {
        self.http = Some(Arc::new(client));
        self
    }

    pub fn signer(mut self, s: impl crate::adapters::signer::ReceiptSigner + 'static) -> Self {
        self.signer = Some(Arc::new(s));
        self
    }

    pub fn llm(mut self, p: impl crate::adapters::llm::LlmProvider + 'static) -> Self {
        self.llm = Some(Arc::new(p));
        self
    }

    pub fn permit_store(mut self, ps: PermitStore) -> Self {
        self.permit_store = Some(Arc::new(ps));
        self
    }

    pub fn build(self) -> DispatchExecutor {
        DispatchExecutor {
            storage_base: self.storage_base,
            #[cfg(feature = "live")]
            http: self.http,
            signer: self.signer,
            llm: self.llm,
            permit_store: self.permit_store,
        }
    }
}

impl DispatchExecutor {
    /// Shorthand: creates a minimal executor with no adapters (stubs only).
    pub fn new(storage_base: impl Into<String>) -> Self {
        DispatchBuilder::new(storage_base).build()
    }

    pub fn builder(storage_base: impl Into<String>) -> DispatchBuilder {
        DispatchBuilder::new(storage_base)
    }

    fn exec_write_storage(&self, path: &str, bytes: &[u8], mime: &str) -> anyhow::Result<()> {
        let full_path = format!("{}/{}", self.storage_base, path);
        if let Some(parent) = std::path::Path::new(&full_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, bytes)?;
        tracing::info!(path = %full_path, mime = %mime, bytes = bytes.len(), "effect.write_storage");
        Ok(())
    }

    fn exec_consent_queue(
        &self,
        ticket_id: &str,
        expires_at: i64,
        required_roles: &[String],
        k: u8,
        n: u8,
        ctx: &ExecCtx,
    ) -> anyhow::Result<()> {
        if let Some(ref _ps) = self.permit_store {
            let ticket = Ticket {
                ticket_id: ticket_id.into(),
                tenant: ctx.tenant.clone(),
                status: TicketStatus::Pending,
                expires_at,
                required_roles: required_roles.to_vec(),
                k,
                n,
                approvals: vec![],
                created_at: ctx.now_nanos,
                closed_at: None,
            };
            // Use PermitStore's internal save via a get+save roundtrip
            // We write directly since PermitStore stores at state_dir/permit-tickets/<tenant>
            let dir = format!("{}/permit-tickets/{}", self.storage_base, ctx.tenant);
            std::fs::create_dir_all(&dir)?;
            let path = format!("{}/{}.json", dir, ticket_id);
            std::fs::write(&path, serde_json::to_string_pretty(&ticket)?)?;
        } else {
            let ticket = serde_json::json!({
                "ticket_id": ticket_id,
                "tenant": ctx.tenant,
                "status": "PENDING",
                "expires_at": expires_at,
                "required_roles": required_roles,
                "k": k,
                "n": n,
                "approvals": [],
                "created_at": ctx.now_nanos,
            });
            let dir = format!("{}/permit-tickets/{}", self.storage_base, ctx.tenant);
            std::fs::create_dir_all(&dir)?;
            let path = format!("{}/{}.json", dir, ticket_id);
            std::fs::write(&path, serde_json::to_string_pretty(&ticket)?)?;
        }
        tracing::info!(ticket_id = %ticket_id, k = k, n = n, "effect.consent.queue");
        Ok(())
    }

    fn exec_consent_close(
        &self,
        ticket_id: &str,
        outcome: &str,
        ctx: &ExecCtx,
    ) -> anyhow::Result<()> {
        let path = format!(
            "{}/permit-tickets/{}/{}.json",
            self.storage_base, ctx.tenant, ticket_id
        );
        if std::path::Path::new(&path).exists() {
            let raw = std::fs::read_to_string(&path)?;
            let mut ticket: serde_json::Value = serde_json::from_str(&raw)?;
            ticket["status"] = serde_json::Value::String(outcome.into());
            ticket["closed_at"] = serde_json::Value::Number(ctx.now_nanos.into());
            std::fs::write(&path, serde_json::to_string_pretty(&ticket)?)?;
        }
        tracing::info!(ticket_id = %ticket_id, outcome = %outcome, "effect.consent.close");
        Ok(())
    }
}

#[async_trait]
impl EffectExecutor for DispatchExecutor {
    async fn execute(&self, effect: &Effect, ctx: &ExecCtx) -> anyhow::Result<()> {
        match effect {
            Effect::Webhook {
                url,
                body,
                content_type,
                hmac_key_env,
            } => {
                let resolved_url =
                    crate::bindings::resolve(&ctx.io_bindings, url).unwrap_or_else(|_| url.clone());

                #[cfg(feature = "live")]
                if let Some(ref http) = self.http {
                    let hmac_secret = hmac_key_env
                        .as_ref()
                        .and_then(|k| crate::bindings::resolve(&ctx.io_bindings, k).ok());
                    let idem = idempotency_key(&ctx.capsule_id_hex, &ctx.step_id, &resolved_url);
                    let outcome = http
                        .post(
                            &resolved_url,
                            body,
                            content_type,
                            hmac_secret.as_deref(),
                            &idem,
                        )
                        .await?;
                    tracing::info!(
                        url = %resolved_url,
                        status = outcome.status,
                        retries = outcome.retries,
                        latency_ms = outcome.latency_ms,
                        "effect.webhook"
                    );
                    if outcome.status >= 400 {
                        return Err(anyhow::anyhow!("webhook failed: HTTP {}", outcome.status));
                    }
                    return Ok(());
                }

                tracing::info!(
                    url = %resolved_url,
                    content_type = %content_type,
                    body_len = body.len(),
                    has_hmac = hmac_key_env.is_some(),
                    "effect.webhook (no http adapter)"
                );
                Ok(())
            }

            Effect::WriteStorage { path, bytes, mime } => {
                self.exec_write_storage(path, bytes, mime)
            }

            Effect::QueueConsentTicket {
                ticket_id,
                expires_at,
                required_roles,
                k,
                n,
            } => self.exec_consent_queue(ticket_id, *expires_at, required_roles, *k, *n, ctx),

            Effect::CloseConsentTicket { ticket_id, outcome } => {
                self.exec_consent_close(ticket_id, outcome, ctx)
            }

            Effect::AppendReceipt {
                payload_nrf,
                signer_binding,
            } => {
                if let Some(ref signer) = self.signer {
                    let node_key = crate::bindings::resolve(&ctx.io_bindings, signer_binding)
                        .unwrap_or_else(|_| "unknown-node".into());

                    let prev = {
                        let mut h = blake3::Hasher::new();
                        h.update(payload_nrf);
                        h.finalize().into()
                    };

                    let mut capsule_id = [0u8; 32];
                    if let Ok(bytes) = hex::decode(&ctx.capsule_id_hex) {
                        if bytes.len() == 32 {
                            capsule_id.copy_from_slice(&bytes);
                        }
                    }

                    let draft = crate::adapters::signer::HopDraft {
                        capsule_id,
                        prev,
                        kind: "step".into(),
                        node: node_key,
                        ts: ctx.now_nanos,
                    };

                    let receipt = signer.sign_hop(&draft)?;
                    tracing::info!(
                        receipt_id = %hex::encode(receipt.id),
                        signer_binding = %signer_binding,
                        "effect.append_receipt"
                    );
                    return Ok(());
                }

                tracing::info!(
                    payload_len = payload_nrf.len(),
                    signer_binding = %signer_binding,
                    "effect.append_receipt (no signer adapter)"
                );
                Ok(())
            }

            Effect::RelayOut {
                to,
                url_binding,
                body,
            } => {
                let resolved_url = crate::bindings::resolve(&ctx.io_bindings, url_binding)
                    .unwrap_or_else(|_| url_binding.clone());

                #[cfg(feature = "live")]
                if let Some(ref http) = self.http {
                    let idem = idempotency_key(&ctx.capsule_id_hex, &ctx.step_id, to);
                    let outcome = http
                        .post(&resolved_url, body, "application/octet-stream", None, &idem)
                        .await?;
                    tracing::info!(
                        to = %to,
                        url = %resolved_url,
                        status = outcome.status,
                        retries = outcome.retries,
                        latency_ms = outcome.latency_ms,
                        "effect.relay_out"
                    );
                    if outcome.status >= 400 {
                        return Err(anyhow::anyhow!("relay_out failed: HTTP {}", outcome.status));
                    }
                    return Ok(());
                }

                tracing::info!(
                    to = %to,
                    url = %resolved_url,
                    body_len = body.len(),
                    "effect.relay_out (no http adapter)"
                );
                Ok(())
            }

            Effect::InvokeLlm {
                model_binding,
                prompt,
                max_tokens,
                cache_key,
            } => {
                if let Some(ref llm) = self.llm {
                    let model = crate::bindings::resolve(&ctx.io_bindings, model_binding)
                        .unwrap_or_else(|_| model_binding.clone());
                    let output = llm.invoke(&model, prompt, *max_tokens).await?;
                    tracing::info!(
                        model = %model,
                        tokens_used = output.tokens_used,
                        cached = output.cached,
                        cache_key = cache_key.as_deref().unwrap_or("none"),
                        "effect.invoke_llm"
                    );
                    return Ok(());
                }

                tracing::info!(
                    model_binding = %model_binding,
                    prompt_len = prompt.len(),
                    max_tokens = max_tokens,
                    cache_key = cache_key.as_deref().unwrap_or("none"),
                    "effect.invoke_llm (no llm adapter)"
                );
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn effect_kind(e: &Effect) -> &'static str {
    match e {
        Effect::Webhook { .. } => "webhook",
        Effect::WriteStorage { .. } => "write_storage",
        Effect::QueueConsentTicket { .. } => "consent_queue",
        Effect::CloseConsentTicket { .. } => "consent_close",
        Effect::AppendReceipt { .. } => "append_receipt",
        Effect::RelayOut { .. } => "relay_out",
        Effect::InvokeLlm { .. } => "invoke_llm",
    }
}
