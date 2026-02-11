//! LLM provider adapter — binding resolution, deterministic cache,
//! and real providers (OpenAI, Ollama) behind the `live` feature.
//!
//! Cache key: `(prompt_cid, inputs_hash)` → file on disk.
//! Cache hit skips provider call entirely.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Output from an LLM invocation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmOutput {
    pub text: String,
    /// Total tokens (kept for backward compat; = tokens_in + tokens_out).
    pub tokens_used: u32,
    pub cached: bool,
    #[serde(default)]
    pub tokens_in: u32,
    #[serde(default)]
    pub tokens_out: u32,
    #[serde(default)]
    pub cost_usd: f32,
    #[serde(default)]
    pub finish_reason: String,
}

impl LlmOutput {
    /// Convenience constructor for providers that know in/out tokens.
    pub fn new(
        text: String,
        tokens_in: u32,
        tokens_out: u32,
        cost_usd: f32,
        finish_reason: String,
    ) -> Self {
        Self {
            text,
            tokens_used: tokens_in + tokens_out,
            cached: false,
            tokens_in,
            tokens_out,
            cost_usd,
            finish_reason,
        }
    }
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Trait for LLM providers — allows test doubles.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn invoke(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> anyhow::Result<LlmOutput>;

    /// Extended invoke with temperature and json_mode.
    /// Default implementation delegates to the simple `invoke`.
    async fn invoke_ext(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
        _temperature: Option<f32>,
        _json_mode: bool,
    ) -> anyhow::Result<LlmOutput> {
        self.invoke(model, prompt, max_tokens).await
    }
}

// ---------------------------------------------------------------------------
// Config types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ProvidersCfg {
    pub providers: ProviderSet,
    pub defaults: Defaults,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct ProviderSet {
    pub openai: Option<OpenAiCfg>,
    pub ollama: Option<OllamaCfg>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Defaults {
    pub model: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct OpenAiCfg {
    pub enabled: bool,
    pub base_url: String,
    pub api_key_env: String,
    pub models: Vec<String>,
    pub pricing_per_1k: Pricing,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct OllamaCfg {
    pub enabled: bool,
    pub base_url: String,
    pub models: Vec<String>,
    pub pricing_per_1k: Pricing,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Pricing {
    pub input_usd: f32,
    pub output_usd: f32,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

/// Deterministic file-based cache for LLM responses.
pub struct LlmCache {
    base_dir: PathBuf,
}

impl LlmCache {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn cache_path(&self, cache_key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", cache_key))
    }

    /// Try to read a cached response.
    pub fn get(&self, cache_key: &str) -> Option<LlmOutput> {
        let path = self.cache_path(cache_key);
        let data = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    /// Write a response to cache.
    pub fn put(&self, cache_key: &str, output: &LlmOutput) -> anyhow::Result<()> {
        let path = self.cache_path(cache_key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(output)?)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// StubProvider (tests / offline)
// ---------------------------------------------------------------------------

/// Stub provider that returns a fixed response (for tests / offline).
pub struct StubProvider {
    pub response: String,
}

#[async_trait::async_trait]
impl LlmProvider for StubProvider {
    async fn invoke(
        &self,
        _model: &str,
        _prompt: &str,
        _max_tokens: u32,
    ) -> anyhow::Result<LlmOutput> {
        Ok(LlmOutput {
            text: self.response.clone(),
            tokens_used: 0,
            cached: false,
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
            finish_reason: "stop".into(),
        })
    }
}

// ---------------------------------------------------------------------------
// CachedProvider
// ---------------------------------------------------------------------------

/// Cached provider wrapper — checks cache before calling inner provider.
pub struct CachedProvider<P: LlmProvider> {
    inner: P,
    cache: LlmCache,
}

impl<P: LlmProvider> CachedProvider<P> {
    pub fn new(inner: P, cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            inner,
            cache: LlmCache::new(cache_dir),
        }
    }
}

#[async_trait::async_trait]
impl<P: LlmProvider> LlmProvider for CachedProvider<P> {
    async fn invoke(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> anyhow::Result<LlmOutput> {
        self.invoke_ext(model, prompt, max_tokens, None, false).await
    }

    async fn invoke_ext(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        json_mode: bool,
    ) -> anyhow::Result<LlmOutput> {
        let cache_key = {
            let mut h = blake3::Hasher::new();
            h.update(model.as_bytes());
            h.update(prompt.as_bytes());
            h.update(&max_tokens.to_le_bytes());
            hex::encode(h.finalize().as_bytes())
        };

        if let Some(mut cached) = self.cache.get(&cache_key) {
            tracing::debug!(cache_key = %cache_key, "llm.cache.hit");
            cached.cached = true;
            return Ok(cached);
        }

        tracing::debug!(cache_key = %cache_key, "llm.cache.miss");
        let output = self.inner.invoke_ext(model, prompt, max_tokens, temperature, json_mode).await?;
        self.cache.put(&cache_key, &output)?;
        Ok(output)
    }
}

// ---------------------------------------------------------------------------
// OpenAI Provider (feature = "live")
// ---------------------------------------------------------------------------

#[cfg(feature = "live")]
pub mod openai {
    use super::{LlmOutput, LlmProvider};
    use anyhow::Context;
    use reqwest::Client;
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    pub struct OpenAiProvider {
        http: Client,
        base: String,
        api_key: String,
        price_in: f32,
        price_out: f32,
    }

    impl OpenAiProvider {
        pub fn new(base: String, api_key: String, price_in: f32, price_out: f32) -> Self {
            Self {
                http: Client::new(),
                base,
                api_key,
                price_in,
                price_out,
            }
        }
    }

    #[derive(Serialize)]
    struct ChatReq<'a> {
        model: &'a str,
        messages: Vec<Msg<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        response_format: Option<ResponseFormat>,
    }

    #[derive(Serialize)]
    struct Msg<'a> {
        role: &'a str,
        content: &'a str,
    }

    #[derive(Serialize)]
    struct ResponseFormat {
        r#type: String,
    }

    #[derive(Deserialize)]
    struct ChatResp {
        choices: Vec<Choice>,
        usage: Option<Usage>,
    }

    #[derive(Deserialize)]
    struct Choice {
        message: ChoiceMsg,
        finish_reason: Option<String>,
    }

    #[derive(Deserialize)]
    struct ChoiceMsg {
        content: String,
    }

    #[derive(Deserialize)]
    struct Usage {
        prompt_tokens: u32,
        completion_tokens: u32,
    }

    #[async_trait::async_trait]
    impl LlmProvider for OpenAiProvider {
        async fn invoke(
            &self,
            model: &str,
            prompt: &str,
            max_tokens: u32,
        ) -> anyhow::Result<LlmOutput> {
            self.invoke_ext(model, prompt, max_tokens, None, false).await
        }

        async fn invoke_ext(
            &self,
            model: &str,
            prompt: &str,
            max_tokens: u32,
            temperature: Option<f32>,
            json_mode: bool,
        ) -> anyhow::Result<LlmOutput> {
            let url = format!("{}/chat/completions", self.base);
            let req = ChatReq {
                model,
                messages: vec![
                    Msg { role: "system", content: "You are a helpful assistant." },
                    Msg { role: "user", content: prompt },
                ],
                max_tokens: Some(max_tokens),
                temperature,
                response_format: if json_mode {
                    Some(ResponseFormat { r#type: "json_object".into() })
                } else {
                    None
                },
            };

            let resp = self.http.post(&url)
                .bearer_auth(&self.api_key)
                .json(&req)
                .send()
                .await
                .context("openai: http error")?;

            let status = resp.status();
            let body = resp.text().await?;
            anyhow::ensure!(status.is_success(), "openai: HTTP {} body={}", status, body);

            let parsed: ChatResp = serde_json::from_str(&body).context("openai: parse")?;
            let ch = parsed.choices.first().context("openai: empty choices")?;

            let tokens_in = parsed.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0);
            let tokens_out = parsed.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0);
            let cost = (tokens_in as f32 / 1000.0) * self.price_in
                + (tokens_out as f32 / 1000.0) * self.price_out;

            Ok(LlmOutput::new(
                ch.message.content.clone(),
                tokens_in,
                tokens_out,
                cost,
                ch.finish_reason.clone().unwrap_or_else(|| "stop".into()),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Ollama Provider (feature = "live")
// ---------------------------------------------------------------------------

#[cfg(feature = "live")]
pub mod ollama {
    use super::{LlmOutput, LlmProvider};
    use anyhow::Context;
    use reqwest::Client;
    use serde::{Deserialize, Serialize};

    #[derive(Clone)]
    pub struct OllamaProvider {
        http: Client,
        base: String,
        price_in: f32,
        price_out: f32,
    }

    impl OllamaProvider {
        pub fn new(base: String, price_in: f32, price_out: f32) -> Self {
            Self {
                http: Client::new(),
                base,
                price_in,
                price_out,
            }
        }
    }

    #[derive(Serialize)]
    struct ChatReq<'a> {
        model: &'a str,
        messages: Vec<Msg<'a>>,
        stream: bool,
        options: Options,
    }

    #[derive(Serialize)]
    struct Msg<'a> {
        role: &'a str,
        content: &'a str,
    }

    #[derive(Serialize, Default)]
    struct Options {
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        num_predict: Option<i32>,
    }

    #[derive(Deserialize)]
    struct ChatResp {
        message: MsgOwned,
        prompt_eval_count: Option<u32>,
        eval_count: Option<u32>,
        done_reason: Option<String>,
    }

    #[derive(Deserialize)]
    struct MsgOwned {
        #[allow(dead_code)]
        role: String,
        content: String,
    }

    #[async_trait::async_trait]
    impl LlmProvider for OllamaProvider {
        async fn invoke(
            &self,
            model: &str,
            prompt: &str,
            max_tokens: u32,
        ) -> anyhow::Result<LlmOutput> {
            self.invoke_ext(model, prompt, max_tokens, None, false).await
        }

        async fn invoke_ext(
            &self,
            model: &str,
            prompt: &str,
            max_tokens: u32,
            temperature: Option<f32>,
            _json_mode: bool,
        ) -> anyhow::Result<LlmOutput> {
            let url = format!("{}/api/chat", self.base);
            let req = ChatReq {
                model,
                messages: vec![Msg { role: "user", content: prompt }],
                stream: false,
                options: Options {
                    temperature,
                    num_predict: Some(max_tokens as i32),
                },
            };

            let resp = self.http.post(&url)
                .json(&req)
                .send()
                .await
                .context("ollama: http error")?;

            let status = resp.status();
            let body = resp.text().await?;
            anyhow::ensure!(status.is_success(), "ollama: HTTP {} body={}", status, body);

            let parsed: ChatResp = serde_json::from_str(&body).context("ollama: parse")?;
            let tokens_in = parsed.prompt_eval_count.unwrap_or(0);
            let tokens_out = parsed.eval_count.unwrap_or(0);
            let cost = (tokens_in as f32 / 1000.0) * self.price_in
                + (tokens_out as f32 / 1000.0) * self.price_out;

            Ok(LlmOutput::new(
                parsed.message.content,
                tokens_in,
                tokens_out,
                cost,
                parsed.done_reason.unwrap_or_else(|| "stop".into()),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// ProviderMux — routes to the right backend
// ---------------------------------------------------------------------------

#[cfg(feature = "live")]
pub enum ProviderMux {
    OpenAi(std::sync::Arc<openai::OpenAiProvider>),
    Ollama(std::sync::Arc<ollama::OllamaProvider>),
}

#[cfg(feature = "live")]
#[async_trait::async_trait]
impl LlmProvider for ProviderMux {
    async fn invoke(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> anyhow::Result<LlmOutput> {
        self.invoke_ext(model, prompt, max_tokens, None, false).await
    }

    async fn invoke_ext(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
        temperature: Option<f32>,
        json_mode: bool,
    ) -> anyhow::Result<LlmOutput> {
        match self {
            ProviderMux::OpenAi(p) => p.invoke_ext(model, prompt, max_tokens, temperature, json_mode).await,
            ProviderMux::Ollama(p) => p.invoke_ext(model, prompt, max_tokens, temperature, json_mode).await,
        }
    }
}

/// Build a provider from config. Returns None if no provider is enabled.
#[cfg(feature = "live")]
pub fn build_provider(cfg: &ProvidersCfg) -> anyhow::Result<Option<ProviderMux>> {
    if let Some(oc) = cfg.providers.openai.as_ref().filter(|c| c.enabled) {
        let key = std::env::var(&oc.api_key_env)
            .map_err(|_| anyhow::anyhow!("env {} not set", oc.api_key_env))?;
        Ok(Some(ProviderMux::OpenAi(std::sync::Arc::new(
            openai::OpenAiProvider::new(
                oc.base_url.clone(),
                key,
                oc.pricing_per_1k.input_usd,
                oc.pricing_per_1k.output_usd,
            ),
        ))))
    } else if let Some(oc) = cfg.providers.ollama.as_ref().filter(|c| c.enabled) {
        Ok(Some(ProviderMux::Ollama(std::sync::Arc::new(
            ollama::OllamaProvider::new(
                oc.base_url.clone(),
                oc.pricing_per_1k.input_usd,
                oc.pricing_per_1k.output_usd,
            ),
        ))))
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_roundtrip() {
        let dir = std::env::temp_dir().join("ai-nrf1-test-llm-cache");
        let _ = std::fs::remove_dir_all(&dir);
        let cache = LlmCache::new(&dir);

        let output = LlmOutput {
            text: "hello world".into(),
            tokens_used: 42,
            cached: false,
            tokens_in: 10,
            tokens_out: 32,
            cost_usd: 0.001,
            finish_reason: "stop".into(),
        };

        assert!(cache.get("test-key").is_none());
        cache.put("test-key", &output).unwrap();

        let got = cache.get("test-key").unwrap();
        assert_eq!(got.text, "hello world");
        assert_eq!(got.tokens_used, 42);
        assert_eq!(got.tokens_in, 10);
        assert_eq!(got.tokens_out, 32);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn llm_output_new_computes_total() {
        let o = LlmOutput::new("hi".into(), 100, 50, 0.01, "stop".into());
        assert_eq!(o.tokens_used, 150);
        assert_eq!(o.tokens_in, 100);
        assert_eq!(o.tokens_out, 50);
    }
}
