//! cap-llm: deterministic LLM assist — prompt-by-CID, never verdict.
//! See design doc §9F: "cap-llm (Assist)".
//!
//! Resolves an immutable prompt template (by CID), injects context from env,
//! and emits `InvokeLlm` effect for the runtime to call the provider.
//! Output is always artifacts (JSON analysis, HTML), **never** a verdict.

use anyhow::Context;
use modules_core::{Artifact, CapInput, CapOutput, Capability, Cid, Effect};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Config {
    /// Binding name for the model provider (resolved by executor).
    model_binding: String,
    /// CID of the immutable prompt template asset.
    prompt_cid: String,
    /// Map of input names → dot-paths into env.
    #[serde(default)]
    inputs: serde_json::Map<String, Value>,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
    /// What to produce: ["artifact:json:analysis", "effect:webhook"]
    #[serde(default)]
    produce: Vec<String>,
}

fn default_max_tokens() -> u32 {
    512
}

#[derive(Default)]
pub struct LlmModule;

impl LlmModule {
    /// Extract a value from env JSON by dot-path.
    fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
        let mut cur = root;
        for seg in path.split('.') {
            match cur {
                Value::Object(m) => {
                    cur = m.get(seg)?;
                }
                Value::Array(a) => {
                    cur = a.get(seg.parse::<usize>().ok()?)?;
                }
                _ => return None,
            }
        }
        Some(cur)
    }

    /// Render prompt template by replacing `{{key}}` with extracted values.
    fn render_prompt(template: &str, env_json: &Value, inputs: &serde_json::Map<String, Value>) -> String {
        let mut rendered = template.to_string();
        for (key, path_val) in inputs {
            if let Some(path_str) = path_val.as_str() {
                let replacement = Self::get(env_json, path_str)
                    .map(|v| match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_else(|| "<missing>".into());
                rendered = rendered.replace(&format!("{{{{{key}}}}}"), &replacement);
            }
        }
        rendered
    }

    /// Compute cache key: blake3(prompt_cid || sorted_input_values).
    fn cache_key(prompt_cid: &str, env_json: &Value, inputs: &serde_json::Map<String, Value>) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(prompt_cid.as_bytes());
        for (key, path_val) in inputs {
            hasher.update(key.as_bytes());
            if let Some(path_str) = path_val.as_str() {
                let val = Self::get(env_json, path_str)
                    .map(|v| v.to_string())
                    .unwrap_or_default();
                hasher.update(val.as_bytes());
            }
        }
        hex::encode(hasher.finalize().as_bytes())
    }
}

impl Capability for LlmModule {
    fn kind(&self) -> &'static str {
        "cap-llm"
    }
    fn api_version(&self) -> &'static str {
        "0.1.0"
    }

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let cfg: Config =
            serde_json::from_value(config.clone()).context("invalid cap-llm config")?;
        anyhow::ensure!(!cfg.model_binding.is_empty(), "model_binding required");
        anyhow::ensure!(!cfg.prompt_cid.is_empty(), "prompt_cid required");
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;

        // Resolve prompt template from assets by CID.
        let prompt_cid_bytes: Cid = {
            let decoded = hex::decode(cfg.prompt_cid.strip_prefix("b3:").unwrap_or(&cfg.prompt_cid))
                .unwrap_or_else(|_| vec![0u8; 32]);
            let mut arr = [0u8; 32];
            let len = decoded.len().min(32);
            arr[..len].copy_from_slice(&decoded[..len]);
            arr
        };

        let prompt_template = match input.assets.get(&prompt_cid_bytes) {
            Ok(asset) => String::from_utf8_lossy(&asset.bytes).into_owned(),
            Err(_) => {
                // Fallback: use prompt_cid as a placeholder template.
                format!("Analyze the following context: {{{{summary}}}}")
            }
        };

        // Convert env to JSON for path extraction.
        let env_json = ubl_json_view::to_json(&input.env);

        // Render prompt.
        let rendered = Self::render_prompt(&prompt_template, &env_json, &cfg.inputs);

        // Cache key for determinism.
        let cache_key = Self::cache_key(&cfg.prompt_cid, &env_json, &cfg.inputs);

        let mut artifacts = vec![];
        let mut effects = vec![];

        // Always emit InvokeLlm effect (the executor calls the provider).
        effects.push(Effect::InvokeLlm {
            model_binding: cfg.model_binding.clone(),
            prompt: rendered.clone(),
            max_tokens: cfg.max_tokens,
            cache_key: Some(cache_key),
        });

        // Produce artifacts as requested.
        for p in &cfg.produce {
            if p.starts_with("artifact:json:") {
                let name = p.strip_prefix("artifact:json:").unwrap_or("analysis");
                let placeholder = serde_json::json!({
                    "status": "pending",
                    "model": cfg.model_binding,
                    "prompt_cid": cfg.prompt_cid,
                    "note": "LLM response will be filled by executor"
                });
                artifacts.push(Artifact {
                    cid: None,
                    mime: "application/json".into(),
                    bytes: serde_json::to_vec_pretty(&placeholder).unwrap_or_default(),
                    name: Some(format!("{name}.json")),
                });
            }
            if p == "effect:webhook" {
                effects.push(Effect::Webhook {
                    url: "<binding:webhook.url>".into(),
                    body: rendered.as_bytes().to_vec(),
                    content_type: "text/plain".into(),
                    hmac_key_env: None,
                });
            }
        }

        // NEVER set verdict — design doc rule §9F.
        Ok(CapOutput {
            artifacts,
            effects,
            metrics: vec![
                ("prompt_len".into(), rendered.len() as i64),
                ("max_tokens".into(), cfg.max_tokens as i64),
            ],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modules_core::{Asset, AssetResolver, ExecutionMeta};

    #[derive(Clone)]
    struct MockResolver {
        prompt: String,
    }
    impl AssetResolver for MockResolver {
        fn get(&self, _cid: &Cid) -> anyhow::Result<Asset> {
            Ok(Asset {
                cid: [0u8; 32],
                bytes: self.prompt.as_bytes().to_vec(),
                mime: "text/plain".into(),
            })
        }
        fn box_clone(&self) -> Box<dyn AssetResolver> {
            Box::new(self.clone())
        }
    }

    fn make_input(config: Value, prompt: &str) -> CapInput {
        CapInput {
            env: nrf1::Value::Map({
                let mut m = std::collections::BTreeMap::new();
                m.insert("summary".into(), nrf1::Value::String("test document text".into()));
                m
            }),
            config,
            assets: Box::new(MockResolver { prompt: prompt.into() }),
            prev_receipts: vec![],
            meta: ExecutionMeta {
                run_id: "run-llm-001".into(),
                tenant: None,
                trace_id: None,
                ts_nanos: 1_700_000_000_000_000_000,
            },
        }
    }

    #[test]
    fn validate_ok() {
        let m = LlmModule;
        let cfg = serde_json::json!({
            "model_binding": "OPENAI_GPT4O_MINI",
            "prompt_cid": "b3:0000000000000000000000000000000000000000000000000000000000000000",
            "inputs": { "summary": "summary" },
            "produce": ["artifact:json:analysis"]
        });
        assert!(m.validate_config(&cfg).is_ok());
    }

    #[test]
    fn validate_empty_model_fails() {
        let m = LlmModule;
        let cfg = serde_json::json!({
            "model_binding": "",
            "prompt_cid": "b3:abc"
        });
        assert!(m.validate_config(&cfg).is_err());
    }

    #[test]
    fn execute_never_sets_verdict() {
        let m = LlmModule;
        let cfg = serde_json::json!({
            "model_binding": "TEST_MODEL",
            "prompt_cid": "b3:0000000000000000000000000000000000000000000000000000000000000000",
            "inputs": { "summary": "summary" },
            "produce": ["artifact:json:analysis"]
        });
        let out = m.execute(make_input(cfg, "Analyze: {{summary}}")).unwrap();

        assert!(out.verdict.is_none(), "cap-llm must NEVER set verdict");
        assert_eq!(out.artifacts.len(), 1);
        assert!(out.effects.iter().any(|e| matches!(e, Effect::InvokeLlm { .. })));
    }

    #[test]
    fn prompt_rendering() {
        let env_json = serde_json::json!({"doc": {"text": "hello world"}});
        let mut inputs = serde_json::Map::new();
        inputs.insert("content".into(), Value::String("doc.text".into()));

        let rendered = LlmModule::render_prompt("Say: {{content}}", &env_json, &inputs);
        assert_eq!(rendered, "Say: hello world");
    }

    #[test]
    fn cache_key_deterministic() {
        let env = serde_json::json!({"x": 42});
        let mut inputs = serde_json::Map::new();
        inputs.insert("val".into(), Value::String("x".into()));

        let k1 = LlmModule::cache_key("cid1", &env, &inputs);
        let k2 = LlmModule::cache_key("cid1", &env, &inputs);
        assert_eq!(k1, k2);

        let k3 = LlmModule::cache_key("cid2", &env, &inputs);
        assert_ne!(k1, k3);
    }
}
