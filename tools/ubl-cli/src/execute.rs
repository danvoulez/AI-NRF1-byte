//! Manifest execution: uses real `module-runner` pipeline with feature `runner-real`,
//! or falls back to a deterministic stub for DX.
//!
//! With `runner-real`: registers all 8 capability modules, translates the YAML manifest
//! into module-runner's Manifest format, runs the pipeline with LoggingExecutor.

use anyhow::{Result, Context};
use serde_json::json;

/// Translate our YAML manifest format into module-runner's Manifest struct.
/// Our format: { version, name, pipeline: [{ use, with }] }
/// Runner format: { v, name, version, pipeline: [{ step_id, kind, version, config }] }
fn yaml_to_runner_manifest(manifest: &serde_yaml::Value) -> Result<serde_json::Value> {
    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("pipeline");
    let mut steps = vec![];

    if let Some(pipeline) = manifest.get("pipeline").and_then(|v| v.as_sequence()) {
        for (i, step) in pipeline.iter().enumerate() {
            let kind = step.get("use").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("step #{i} missing 'use'"))?;

            // Resolve aliases: policy.light → cap-policy, cap-llm-engine → cap-llm, etc.
            let resolved_kind = resolve_alias(kind);

            let config = step.get("with")
                .map(|v| serde_json::to_value(v).unwrap_or_default())
                .unwrap_or(json!({}));

            steps.push(json!({
                "step_id": format!("step-{i}-{kind}"),
                "kind": resolved_kind,
                "version": "*",
                "config": config
            }));
        }
    }

    Ok(json!({
        "v": "1",
        "name": name,
        "version": "1.0.0",
        "pipeline": steps
    }))
}

/// Map manifest capability aliases to real module kinds.
fn resolve_alias(kind: &str) -> &str {
    match kind {
        "policy.light" | "policy.hardening" => "cap-policy",
        "cap-structure" => "cap-intake",
        "cap-llm-engine" | "cap-llm-smart" => "cap-llm",
        other => other,
    }
}

/// Build the CapRegistry with all 8 capability modules.
#[cfg(feature = "runner-real")]
fn build_registry() -> module_runner::cap_registry::CapRegistry {
    let mut reg = module_runner::cap_registry::CapRegistry::new();
    reg.register(cap_intake::IntakeModule::default());
    reg.register(cap_policy::PolicyModule::default());
    reg.register(cap_enrich::EnrichModule::default());
    reg.register(cap_transport::TransportModule::default());
    reg.register(cap_permit::PermitModule::default());
    reg.register(cap_llm::LlmModule::default());
    reg.register(cap_pricing::PricingModule::default());
    reg.register(cap_runtime::RuntimeModule::default());
    reg
}

pub async fn run_manifest(manifest: serde_yaml::Value, vars: &[String]) -> Result<serde_json::Value> {
    #[cfg(feature = "runner-real")]
    {
        return run_real_pipeline(manifest, vars).await;
    }

    #[cfg(not(feature = "runner-real"))]
    {
        let _ = vars;
        run_stub(manifest)
    }
}

#[cfg(feature = "runner-real")]
async fn run_real_pipeline(manifest: serde_yaml::Value, vars: &[String]) -> Result<serde_json::Value> {
    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("pipeline").to_string();

    // Translate YAML → runner Manifest
    let manifest_json = yaml_to_runner_manifest(&manifest)?;
    let runner_manifest: module_runner::manifest::Manifest =
        serde_json::from_value(manifest_json).context("failed to parse translated manifest")?;

    // Build initial env from --var flags
    let mut env_map = serde_json::Map::new();
    for kv in vars {
        if let Some((k, v)) = kv.split_once('=') {
            env_map.insert(k.to_string(), serde_json::Value::String(v.to_string()));
        }
    }
    let env_json = serde_json::Value::Object(env_map);
    let env = ubl_json_view::from_json(&env_json)
        .unwrap_or_else(|_| nrf1::Value::Null);

    // Build registry + runner
    let registry = build_registry();
    let assets = module_runner::assets::MemoryResolver::new();
    let effects = module_runner::effects::LoggingExecutor;
    let io_bindings = runner_manifest.io_bindings.clone().unwrap_or(json!({}));

    let runner = module_runner::runner::Runner::new(
        &registry,
        Box::new(assets),
        &effects,
        io_bindings,
        "cli",
    );

    let rt = runner.run(&runner_manifest, env).await?;

    // Format output
    let receipt_cids: Vec<String> = rt.receipts.iter()
        .map(|r| format!("b3:{}", hex::encode(r)))
        .collect();
    let url_rica = receipt_cids.first()
        .map(|cid| format!("https://resolver.local/r/{cid}"))
        .unwrap_or_default();

    let metrics: serde_json::Value = rt.step_metrics.iter()
        .map(|(step, key, val)| json!({"step": step, "metric": key, "value": val}))
        .collect();

    Ok(json!({
        "product": name,
        "verdict": format!("{:?}", rt.verdict),
        "receipt_cid": receipt_cids.first().unwrap_or(&String::new()),
        "receipt_chain": receipt_cids,
        "url_rica": url_rica,
        "artifacts": rt.artifacts.len(),
        "metrics": metrics,
        "stopped_at": rt.stopped_at,
    }))
}

#[cfg(not(feature = "runner-real"))]
fn run_stub(manifest: serde_yaml::Value) -> Result<serde_json::Value> {
    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("pipeline");
    let manifest_json = serde_json::to_string(&manifest)?;
    let hash = blake3::hash(manifest_json.as_bytes());
    let rcid = format!("b3:{}", hex::encode(hash.as_bytes()));
    let url = format!("https://resolver.local/r/{}", rcid);
    Ok(json!({
        "product": name,
        "verdict": "Allow (stub)",
        "receipt_cid": rcid,
        "url_rica": url,
        "artifacts": 0,
        "metrics": {"run_latency_ms": 0},
        "stopped_at": null,
    }))
}
