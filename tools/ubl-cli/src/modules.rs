//! `ubl` subcommands (tdln.* and llm.*) with hardening + execution via module-runner in-process.
//! Real execution is enabled with feature `runner-real`; without it, falls back to deterministic stub.

use clap::Args;
use anyhow::{Result, anyhow};
use std::path::PathBuf;
use std::fs;
use serde_json::json;

use crate::execute; // sibling module

const MANIFEST_MAX_BYTES: usize = 256 * 1024; // 256KB
const MAX_VARS: usize = 50;
const MAX_VAR_KV_LEN: usize = 2048;

static CAP_WHITELIST: &[&str] = &[
    "cap-intake","cap-policy","cap-runtime","cap-structure",
    "cap-llm-engine","cap-llm-smart","cap-enrich","cap-transport","cap-permit",
    "cap-pricing","cap-llm",
    "policy.light","policy.hardening",
];

#[derive(Args, Clone, Debug)]
pub struct RunArgs {
    #[arg(long)]
    pub manifest: Option<PathBuf>,
    #[arg(long, value_name="k=v", num_args=0..)]
    pub var: Vec<String>,
    #[arg(long, default_value = "-")]
    pub out: String,
    #[arg(long, default_value = "configs/schema/manifest.v1.json")]
    pub schema: String,
}

pub async fn run_tdln(kind: &str, args: RunArgs) -> Result<()> {
    let default = match kind {
        "policy" => "manifests/tdln/policy.yaml",
        "runtime" => "manifests/tdln/runtime.yaml",
        _ => return Err(anyhow!("unknown tdln kind '{kind}'. Hint: valid kinds are 'policy' or 'runtime'")),
    };
    run_common(&format!("tdln.{kind}"), args, default).await
}

pub async fn run_llm(kind: &str, args: RunArgs) -> Result<()> {
    let default = match kind {
        "engine" => "manifests/llm/engine.yaml",
        "smart" => "manifests/llm/smart.yaml",
        _ => return Err(anyhow!("unknown llm kind '{kind}'. Hint: valid kinds are 'engine' or 'smart'")),
    };
    run_common(&format!("llm.{kind}"), args, default).await
}

async fn run_common(cmd: &str, mut args: RunArgs, default_manifest: &str) -> Result<()> {
    if args.var.len() > MAX_VARS { anyhow::bail!("too many --var flags (max {}). Hint: reduce the number of variables", MAX_VARS); }
    for kv in &args.var {
        if kv.len() > MAX_VAR_KV_LEN { anyhow::bail!("var too large (> {} bytes). Hint: shorten the value", MAX_VAR_KV_LEN); }
        if !kv.contains('=') { anyhow::bail!("var missing '=': {kv}. Hint: use --var key=value format"); }
    }
    let manifest_path = args.manifest.take().unwrap_or_else(|| PathBuf::from(default_manifest));
    let manifest_txt = read_bounded(&manifest_path, MANIFEST_MAX_BYTES)?;
    let manifest: serde_yaml::Value = serde_yaml::from_str(&manifest_txt)
        .map_err(|e| anyhow!("parse manifest: {e}"))?;

    validate_schema(&args.schema, &manifest)?;
    validate_pipeline(&manifest)?;
    validate_outputs(&manifest, cmd)?;

    let res = execute::run_manifest(manifest.clone(), &args.var).await?;

    let out = json!({
        "ok": true,
        "cmd": cmd,
        "manifest": manifest.get("name").and_then(|v| v.as_str()),
        "result": res
    });
    write_out(&args.out, &out)
}

// ---------- validators & safe IO ----------

fn validate_schema(schema_path: &str, manifest: &serde_yaml::Value) -> Result<()> {
    let schema_file = std::path::Path::new(schema_path);
    if !schema_file.exists() {
        // Schema file not present — skip validation with a warning
        eprintln!("warn: schema file not found at {schema_path}, skipping validation");
        return Ok(());
    }
    let schema_txt = fs::read_to_string(schema_path)?;
    let schema_json: serde_json::Value = serde_json::from_str(&schema_txt)?;
    let compiled = jsonschema::JSONSchema::compile(&schema_json)
        .map_err(|e| anyhow!("schema compile: {e}"))?;
    let doc = serde_json::to_value(manifest)?;
    if let Err(errors) = compiled.validate(&doc) {
        let msgs: Vec<String> = errors
            .map(|e| format!("{} at {}", e, e.instance_path))
            .collect();
        anyhow::bail!("manifest schema validation failed: {}. Hint: check the manifest against the schema at {}", msgs.join("; "), schema_path);
    }
    Ok(())
}

fn validate_pipeline(m: &serde_yaml::Value) -> Result<()> {
    let pipeline = m.get("pipeline").and_then(|v| v.as_sequence())
        .ok_or_else(|| anyhow!("manifest missing 'pipeline' field. Hint: add a 'pipeline' array with at least one step"))?;
    if pipeline.is_empty() { anyhow::bail!("pipeline is empty. Hint: add at least one step with 'use' and 'with' fields"); }
    for (i, step) in pipeline.iter().enumerate() {
        let use_id = step.get("use")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("step #{i} missing 'use' field. Hint: each pipeline step must have 'use: <capability-id>'"))?;
        if !CAP_WHITELIST.iter().any(|cap| cap == &use_id) {
            anyhow::bail!("capability '{}' not allowed in step #{}. Hint: allowed capabilities are: {:?}", use_id, i, CAP_WHITELIST);
        }
    }
    Ok(())
}

fn validate_outputs(m: &serde_yaml::Value, cmd: &str) -> Result<()> {
    if let Some(out) = m.get("outputs") {
        if let Some(fields) = out.get("fields").and_then(|v| v.as_sequence()) {
            let has_receipt = fields.iter().any(|f| f.as_str()==Some("receipt_cid"));
            let has_url = fields.iter().any(|f| f.as_str()==Some("url_rica"));
            if cmd.starts_with("tdln.") || cmd.starts_with("llm.") {
                if !(has_receipt && has_url) {
                    anyhow::bail!("outputs.fields must contain 'receipt_cid' and 'url_rica' for {cmd}. Hint: add both fields to the outputs.fields array in the manifest");
                }
            }
        }
    }
    Ok(())
}

fn read_bounded(p: &PathBuf, max: usize) -> Result<String> {
    safe_manifest_path(p)?;
    let bytes = fs::read(p)?;
    if bytes.len() > max { anyhow::bail!("manifest > {} bytes", max); }
    Ok(String::from_utf8(bytes)?)
}

fn safe_manifest_path(p: &PathBuf) -> Result<()> {
    let s = p.to_string_lossy();
    if s.starts_with('/') || s.contains("..") {
        anyhow::bail!("unsafe manifest path: '{}'. Hint: use a relative path without '..' or leading '/'", s);
    }
    Ok(())
}

fn write_out(path: &str, v: &serde_json::Value) -> Result<()> {
    let s = serde_json::to_string_pretty(v)?;
    if path == "-" { println!("{}", s); } else {
        if std::path::Path::new(path).exists() {
            anyhow::bail!("output file already exists: '{}'. Hint: use a different --out path or delete the existing file", path);
        }
        fs::write(path, s)?;
    }
    Ok(())
}
