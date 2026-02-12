//! cap-runtime HARDENED com allowlist/HMAC/idempotência/nonce-exp.
//!
//! Adapted from pacotão bundle to use the real modules-core CapInput/CapOutput API.

use anyhow::{anyhow, Context, Result};
use modules_core::{CapInput, CapOutput, Capability, Effect, Artifact};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct Limits { cpu_ms: u32, memory_mb: u32, wall_ms: u32 }

#[derive(Debug, Deserialize)]
struct Config {
    executor: String,
    limits: Limits,
    #[serde(default)] code_input: Option<String>,
    #[serde(default)] data_input: Option<String>,
    webhook_binding: String,
    #[serde(default)] executor_allow: Option<Vec<String>>,
    #[serde(default)] hmac_key_env: Option<String>,
    #[serde(default)] max_input_mb: Option<u32>,
}

#[derive(Default)]
pub struct RuntimeModule;

impl Capability for RuntimeModule {
    fn kind(&self) -> &'static str { "cap-runtime" }
    fn api_version(&self) -> &'static str { "1.0" }

    fn validate_config(&self, cfg: &serde_json::Value) -> Result<()> {
        let c: Config = serde_json::from_value(cfg.clone())
            .context("cap-runtime: config inválida")?;
        if c.code_input.is_none() && c.data_input.is_none() {
            return Err(anyhow!("cap-runtime: informe code_input ou data_input"));
        }
        ensure_allowlist(&c.executor, &c.executor_allow)?;
        if let Some(mb) = c.max_input_mb { if mb > 64 { anyhow::bail!("max_input_mb muito alto"); } }
        Ok(())
    }

    fn execute(&self, input: CapInput) -> Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())
            .context("cap-runtime: config inválida")?;
        ensure_allowlist(&cfg.executor, &cfg.executor_allow)?;

        let env_json: Value = ubl_json_view::to_json(&input.env);

        let tenant = input.meta.tenant.as_deref().unwrap_or("default");
        let trace_id = input.meta.trace_id.as_deref().unwrap_or(&input.meta.run_id);
        let step_id = &input.meta.run_id;

        let code_cid = cfg.code_input.as_ref()
            .and_then(|k| env_json.get(k)).and_then(|v| v.get("cid"))
            .and_then(|v| v.as_str()).map(|s| s.to_string());
        let data_cid = cfg.data_input.as_ref()
            .and_then(|k| env_json.get(k)).and_then(|v| v.get("cid"))
            .and_then(|v| v.as_str()).map(|s| s.to_string());

        let nonce = format!("n-{}", trace_id);
        let exp   = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) + 600) as i64;

        let plan = json!({
            "executor": cfg.executor,
            "limits": { "cpu_ms": cfg.limits.cpu_ms, "memory_mb": cfg.limits.memory_mb, "wall_ms": cfg.limits.wall_ms },
            "inputs": { "code_cid": code_cid, "data_cid": data_cid },
            "attestation": { "require_quote": true },
            "idempotency": { "tenant": tenant, "trace_id": trace_id, "step_id": step_id },
            "security": { "nonce": nonce, "exp": exp }
        });
        let plan_bytes = serde_json::to_vec(&plan)?;
        let plan_cid_str = cid_of(&plan_bytes);

        let mut body = json!({
            "tenant": tenant, "trace_id": trace_id, "step_id": step_id,
            "plan_cid": plan_cid_str, "plan": plan
        });

        if let Some(key_env) = &cfg.hmac_key_env {
            if let Ok(k) = std::env::var(key_env) {
                let tag = hmac_sha256::HMAC::mac(serde_json::to_vec(&body)?, k.as_bytes());
                body["hmac"] = serde_json::Value::String(hex::encode(tag));
            }
        }

        // Compute real CID bytes for Artifact
        let plan_cid_bytes: [u8; 32] = *blake3::hash(&plan_bytes).as_bytes();

        let write_effect = Effect::WriteStorage {
            path: format!("runtime-plans/{}/{}.json", tenant, plan_cid_str),
            bytes: plan_bytes.clone(), mime: "application/json".into(),
        };
        let webhook_effect = Effect::Webhook {
            url: format!("${{{}}}", cfg.webhook_binding),
            body: serde_json::to_vec(&body)?, content_type: "application/json".into(),
            hmac_key_env: cfg.hmac_key_env.clone(),
        };

        Ok(CapOutput{
            new_env: None, verdict: None,
            artifacts: vec![Artifact{
                cid: Some(plan_cid_bytes),
                mime: "application/json".into(),
                bytes: plan_bytes,
                name: Some("runtime-plan.json".into()),
            }],
            effects: vec![write_effect, webhook_effect],
            metrics: vec![("runtime.request".into(), 1)],
        })
    }
}

fn ensure_allowlist(exec: &str, allow: &Option<Vec<String>>) -> Result<()> {
    if let Some(list) = allow {
        if !list.iter().any(|x| x == exec) {
            return Err(anyhow!("executor não permitido: {}", exec));
        }
    }
    Ok(())
}

fn cid_of(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    format!("b3-{}", hex::encode(hash.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_missing_inputs() {
        let m = RuntimeModule;
        let cfg = json!({
            "executor": "wasmtime",
            "limits": {"cpu_ms": 100, "memory_mb": 64, "wall_ms": 200},
            "webhook_binding": "EXEC_URL"
        });
        assert!(m.validate_config(&cfg).is_err());
    }

    #[test]
    fn validate_rejects_blocked_executor() {
        let m = RuntimeModule;
        let cfg = json!({
            "executor": "evil-exec",
            "limits": {"cpu_ms": 100, "memory_mb": 64, "wall_ms": 200},
            "code_input": "code",
            "webhook_binding": "EXEC_URL",
            "executor_allow": ["wasmtime"]
        });
        assert!(m.validate_config(&cfg).is_err());
    }

    #[test]
    fn validate_accepts_good_config() {
        let m = RuntimeModule;
        let cfg = json!({
            "executor": "wasmtime",
            "limits": {"cpu_ms": 100, "memory_mb": 64, "wall_ms": 200},
            "code_input": "code",
            "webhook_binding": "EXEC_URL",
            "executor_allow": ["wasmtime"]
        });
        assert!(m.validate_config(&cfg).is_ok());
    }
}
