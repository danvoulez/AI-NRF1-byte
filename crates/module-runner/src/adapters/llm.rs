//! LLM provider adapter — binding resolution, deterministic cache.
//!
//! Cache key: `(prompt_cid, inputs_hash)` → file on disk.
//! Cache hit skips provider call entirely.

use std::path::PathBuf;

/// Output from an LLM invocation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmOutput {
    pub text: String,
    pub tokens_used: u32,
    pub cached: bool,
}

/// Trait for LLM providers — allows test doubles.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn invoke(
        &self,
        model: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> anyhow::Result<LlmOutput>;
}

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
        })
    }
}

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
        let output = self.inner.invoke(model, prompt, max_tokens).await?;
        self.cache.put(&cache_key, &output)?;
        Ok(output)
    }
}

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
        };

        assert!(cache.get("test-key").is_none());
        cache.put("test-key", &output).unwrap();

        let got = cache.get("test-key").unwrap();
        assert_eq!(got.text, "hello world");
        assert_eq!(got.tokens_used, 42);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
