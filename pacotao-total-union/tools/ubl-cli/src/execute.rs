//! Abstração de execução do manifesto: usa `module-runner` real com feature `runner-real`
//! ou cai num stub determinístico que simula receipt/url para DX.

use anyhow::Result;
use serde_json::json;

pub fn run_manifest(manifest: serde_yaml::Value, _vars: &[(String,String)]) -> Result<serde_json::Value> {
    #[cfg(feature = "runner-real")]
    {
        // Integração "best effort" com module-runner (ajuste conforme API real do repo).
        use module_runner_inprocess::{InProcess, RunRequest, RunResponse};
        let req = RunRequest { manifest: serde_json::to_value(&manifest)?, vars: serde_json::Map::new() };
        let res: RunResponse = InProcess::run(req)?;
        return Ok(serde_json::to_value(res)?);
    }
    #[cfg(not(feature = "runner-real"))]
    {
        // Stub determinístico: emite fields essenciais para DX (receipt_cid + url_rica)
        let name = manifest.get("name").and_then(|v| v.as_str()).unwrap_or("pipeline");
        let rcid = format!("bafyR-{}", blake3::hash(serde_json::to_string(&manifest)?.as_bytes()).to_hex());
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
}

// Dummy blake3 helper for the stub path
#[cfg(not(feature = "runner-real"))]
mod blake3 {
    pub fn hash(b: &[u8]) -> blake3::Hash { blake3::hash(b) }
    pub trait ToHex { fn to_hex(&self) -> String; }
    impl ToHex for blake3::Hash {
        fn to_hex(&self) -> String { hex::encode(self.as_bytes()) }
    }
}
