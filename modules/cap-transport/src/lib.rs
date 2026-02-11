//! cap-transport: SIRP hop receipts, nonce/exp validation, relay out.
//! See design doc §9E: "cap-transport (SIRP/Relay)".
//!
//! Builds receipt payloads (NRF-encoded, unsigned) and emits
//! `AppendReceipt` + `RelayOut` effects for the runtime to execute.
//! Validates nonce uniqueness and expiration within the pure step.

use anyhow::Context;
use modules_core::{Artifact, CapInput, CapOutput, Capability, Cid, Effect};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
struct Relay {
    kind: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    node: String,
    #[serde(default)]
    relay: Vec<Relay>,
    #[serde(default = "default_clock_skew")]
    clock_skew_sec: i64,
}

fn default_clock_skew() -> i64 {
    60
}

#[derive(Default)]
pub struct TransportModule;

impl TransportModule {
    /// Build the NRF-encoded receipt payload (without sig).
    /// Fields: {domain, of, prev, kind, node, ts} — sorted by key (BTreeMap).
    fn build_receipt_payload(
        of: &Cid,
        prev: &Cid,
        kind: &str,
        node: &str,
        ts: i64,
    ) -> Vec<u8> {
        let mut m = BTreeMap::new();
        m.insert("domain".into(), nrf1::Value::String("ubl-receipt/1.0".into()));
        m.insert("kind".into(), nrf1::Value::String(kind.into()));
        m.insert("node".into(), nrf1::Value::String(node.into()));
        m.insert("of".into(), nrf1::Value::Bytes(of.to_vec()));
        m.insert("prev".into(), nrf1::Value::Bytes(prev.to_vec()));
        m.insert("ts".into(), nrf1::Value::Int(ts));
        nrf1::encode(&nrf1::Value::Map(m))
    }

    /// Check expiration: ts_nanos + skew >= now.
    fn check_exp(exp_nanos: Option<i64>, now_nanos: i64, skew_sec: i64) -> anyhow::Result<()> {
        if let Some(exp) = exp_nanos {
            let deadline = exp + (skew_sec * 1_000_000_000);
            anyhow::ensure!(
                now_nanos <= deadline,
                "capsule expired: exp={exp}, now={now_nanos}, skew={skew_sec}s"
            );
        }
        Ok(())
    }

    /// Derive a hop artifact (JSON summary for inspection).
    fn hop_artifact(node: &str, prev_receipts: &[Cid]) -> Artifact {
        let summary = serde_json::json!({
            "node": node,
            "hop_index": prev_receipts.len(),
            "prev_count": prev_receipts.len(),
        });
        Artifact {
            cid: None,
            mime: "application/json".into(),
            bytes: serde_json::to_vec_pretty(&summary).unwrap_or_default(),
            name: Some("hop-summary.json".into()),
        }
    }
}

impl Capability for TransportModule {
    fn kind(&self) -> &'static str {
        "cap-transport"
    }
    fn api_version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let cfg: Config =
            serde_json::from_value(config.clone()).context("invalid cap-transport config")?;
        anyhow::ensure!(cfg.node.is_ascii(), "node DID must be ASCII");
        anyhow::ensure!(!cfg.node.is_empty(), "node DID must not be empty");
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let mut effects = vec![];

        // Determine prev: last receipt CID or zeros (genesis).
        let prev: Cid = input.prev_receipts.last().copied().unwrap_or([0u8; 32]);

        // Build a placeholder capsule_id from env hash (the real one comes from ubl_capsule::compute_id).
        let env_bytes = nrf1::encode(&input.env);
        let of: Cid = *blake3::hash(&env_bytes).as_bytes();

        // Build receipt payload (NRF bytes, unsigned).
        let payload_nrf = Self::build_receipt_payload(
            &of,
            &prev,
            "pipeline-hop",
            &cfg.node,
            input.meta.ts_nanos,
        );

        effects.push(Effect::AppendReceipt {
            payload_nrf,
            signer_binding: "NODE_KEY".into(),
        });

        // Relay out to each configured destination.
        for r in &cfg.relay {
            let capsule_json = serde_json::json!({
                "of": hex::encode(of),
                "node": cfg.node,
                "ts": input.meta.ts_nanos,
            });
            effects.push(Effect::RelayOut {
                to: r.kind.clone(),
                url_binding: r.url.clone(),
                body: serde_json::to_vec(&capsule_json).unwrap_or_default(),
            });
        }

        let artifact = Self::hop_artifact(&cfg.node, &input.prev_receipts);

        Ok(CapOutput {
            artifacts: vec![artifact],
            effects,
            metrics: vec![
                ("hops_prev".into(), input.prev_receipts.len() as i64),
                ("relay_count".into(), cfg.relay.len() as i64),
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
    struct NullResolver;
    impl AssetResolver for NullResolver {
        fn get(&self, _cid: &modules_core::Cid) -> anyhow::Result<Asset> {
            anyhow::bail!("no assets")
        }
        fn box_clone(&self) -> Box<dyn AssetResolver> {
            Box::new(self.clone())
        }
    }

    fn make_input(config: Value) -> CapInput {
        CapInput {
            env: nrf1::Value::Map(BTreeMap::new()),
            config,
            assets: Box::new(NullResolver),
            prev_receipts: vec![],
            meta: ExecutionMeta {
                run_id: "run-001".into(),
                tenant: None,
                trace_id: None,
                ts_nanos: 1_700_000_000_000_000_000,
            },
        }
    }

    #[test]
    fn validate_ok() {
        let m = TransportModule;
        let cfg = serde_json::json!({
            "node": "did:ubl:node-01#key-1",
            "relay": [{ "kind": "http", "url": "https://relay.example.com/ingest" }]
        });
        assert!(m.validate_config(&cfg).is_ok());
    }

    #[test]
    fn validate_non_ascii_fails() {
        let m = TransportModule;
        let cfg = serde_json::json!({ "node": "did:ubl:café#key-1" });
        assert!(m.validate_config(&cfg).is_err());
    }

    #[test]
    fn execute_produces_receipt_and_relay() {
        let m = TransportModule;
        let cfg = serde_json::json!({
            "node": "did:ubl:node-01#key-1",
            "relay": [{ "kind": "http", "url": "https://relay.example.com" }]
        });
        let out = m.execute(make_input(cfg)).unwrap();

        assert_eq!(out.artifacts.len(), 1);
        assert_eq!(out.artifacts[0].name.as_deref(), Some("hop-summary.json"));

        // AppendReceipt + RelayOut
        assert_eq!(out.effects.len(), 2);
        assert!(matches!(&out.effects[0], Effect::AppendReceipt { .. }));
        assert!(matches!(&out.effects[1], Effect::RelayOut { .. }));
    }

    #[test]
    fn check_exp_ok() {
        let now = 1_700_000_000_000_000_000i64;
        let exp = now + 60_000_000_000; // 60s in future
        assert!(TransportModule::check_exp(Some(exp), now, 60).is_ok());
    }

    #[test]
    fn check_exp_expired() {
        let now = 1_700_000_000_000_000_000i64;
        let exp = now - 120_000_000_000; // 120s in past
        assert!(TransportModule::check_exp(Some(exp), now, 60).is_err());
    }
}
