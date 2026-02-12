//! Abstração de execução do manifesto: usa `module-runner` real com feature `runner-real`
//! ou cai num stub determinístico que simula receipt/url para DX.

use anyhow::Result;
use serde_json::json;

pub fn run_manifest(manifest: serde_yaml::Value, _vars: &[String]) -> Result<serde_json::Value> {
    #[cfg(feature = "runner-real")]
    {
        // Integração "best effort" com module-runner (ajuste conforme API real do repo).
        // TODO: wire up real module-runner InProcess execution when ready.
        // For now, fall through to stub even with the feature enabled.
        let _ = &manifest;
    }

    // Stub determinístico: emite fields essenciais para DX (receipt_cid + url_rica)
    let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("pipeline");
    let manifest_json = serde_json::to_string(&manifest)?;
    let hash = blake3::hash(manifest_json.as_bytes());
    let rcid = format!("bafyR-{}", hex::encode(hash.as_bytes()));
    let url = format!("https://resolver.local/r/{}", rcid);
    let out = json!({
        "product": name,
        "receipt_cid": rcid,
        "url_rica": url,
        "artifacts": [],
        "metrics": {"run_latency_ms": 0}
    });
    Ok(out)
}
