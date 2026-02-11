//! cap-permit: K-of-N consent flow with ticket lifecycle.
//! See design doc §9C: "cap-permit (Consent)".
//!
//! When policy returns `REQUIRE`, this module opens a consent ticket,
//! emits `QueueConsentTicket` effect, and produces an HTML artifact
//! showing ticket status. The runtime's EffectExecutor handles actual
//! storage and approval collection.

use anyhow::Context;
use modules_core::{Artifact, CapInput, CapOutput, Capability, Effect, Verdict};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Quorum {
    k: u8,
    n: u8,
    #[serde(default)]
    roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Config {
    quorum: Quorum,
    #[serde(default = "default_ttl")]
    ttl_sec: i64,
    #[serde(default = "default_timeout_action")]
    timeout_action: String,
}

fn default_ttl() -> i64 {
    3600
}
fn default_timeout_action() -> String {
    "DENY".into()
}

#[derive(Default)]
pub struct PermitModule;

impl PermitModule {
    fn ticket_id(meta: &modules_core::ExecutionMeta) -> String {
        format!("ticket-{}", meta.run_id)
    }

    fn status_html(ticket_id: &str, cfg: &Config) -> Artifact {
        let html = format!(
            r#"<!doctype html>
<html>
<head><meta charset="utf-8"><title>Consent Ticket — {tid}</title></head>
<body>
  <h1>Consent Required</h1>
  <p>Ticket: <code>{tid}</code></p>
  <p>Quorum: {k} of {n} (roles: {roles})</p>
  <p>TTL: {ttl}s — timeout action: {ta}</p>
  <p>Status: <strong>PENDING</strong></p>
</body>
</html>"#,
            tid = ticket_id,
            k = cfg.quorum.k,
            n = cfg.quorum.n,
            roles = cfg.quorum.roles.join(", "),
            ttl = cfg.ttl_sec,
            ta = cfg.timeout_action,
        );
        Artifact {
            cid: None,
            mime: "text/html".into(),
            bytes: html.into_bytes(),
            name: Some("consent-ticket.html".into()),
        }
    }
}

impl Capability for PermitModule {
    fn kind(&self) -> &'static str {
        "cap-permit"
    }
    fn api_version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let cfg: Config =
            serde_json::from_value(config.clone()).context("invalid cap-permit config")?;
        anyhow::ensure!(cfg.quorum.k > 0, "quorum.k must be > 0");
        anyhow::ensure!(cfg.quorum.k <= cfg.quorum.n, "quorum.k must be <= n");
        anyhow::ensure!(
            cfg.quorum.roles.len() == cfg.quorum.n as usize,
            "roles.len() must equal quorum.n"
        );
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let ticket_id = Self::ticket_id(&input.meta);
        let expires_at = input.meta.ts_nanos + (cfg.ttl_sec * 1_000_000_000);

        let artifact = Self::status_html(&ticket_id, &cfg);

        let effects = vec![
            Effect::QueueConsentTicket {
                ticket_id: ticket_id.clone(),
                expires_at,
                required_roles: cfg.quorum.roles.clone(),
                k: cfg.quorum.k,
                n: cfg.quorum.n,
            },
            Effect::WriteStorage {
                path: format!("consent/{}.html", ticket_id),
                bytes: artifact.bytes.clone(),
                mime: "text/html".into(),
            },
        ];

        Ok(CapOutput {
            verdict: Some(Verdict::Require),
            artifacts: vec![artifact],
            effects,
            metrics: vec![
                ("consent_k".into(), cfg.quorum.k as i64),
                ("consent_n".into(), cfg.quorum.n as i64),
            ],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modules_core::ExecutionMeta;

    fn test_meta() -> ExecutionMeta {
        ExecutionMeta {
            run_id: "test-run-001".into(),
            tenant: Some("acme".into()),
            trace_id: None,
            ts_nanos: 1_700_000_000_000_000_000,
        }
    }

    fn test_config() -> Value {
        serde_json::json!({
            "quorum": { "k": 2, "n": 3, "roles": ["ops", "risk", "legal"] },
            "ttl_sec": 1800,
            "timeout_action": "DENY"
        })
    }

    fn make_input(config: Value) -> CapInput {
        use modules_core::{Asset, AssetResolver, Cid};

        #[derive(Clone)]
        struct NullResolver;
        impl AssetResolver for NullResolver {
            fn get(&self, _cid: &Cid) -> anyhow::Result<Asset> {
                anyhow::bail!("no assets")
            }
            fn box_clone(&self) -> Box<dyn AssetResolver> {
                Box::new(self.clone())
            }
        }

        CapInput {
            env: nrf1::Value::Null,
            config,
            assets: Box::new(NullResolver),
            prev_receipts: vec![],
            meta: test_meta(),
        }
    }

    #[test]
    fn validate_ok() {
        let m = PermitModule;
        assert!(m.validate_config(&test_config()).is_ok());
    }

    #[test]
    fn validate_k_zero_fails() {
        let m = PermitModule;
        let cfg = serde_json::json!({
            "quorum": { "k": 0, "n": 2, "roles": ["a", "b"] }
        });
        assert!(m.validate_config(&cfg).is_err());
    }

    #[test]
    fn validate_k_gt_n_fails() {
        let m = PermitModule;
        let cfg = serde_json::json!({
            "quorum": { "k": 3, "n": 2, "roles": ["a", "b"] }
        });
        assert!(m.validate_config(&cfg).is_err());
    }

    #[test]
    fn execute_produces_ticket() {
        let m = PermitModule;
        let input = make_input(test_config());
        let out = m.execute(input).unwrap();

        assert_eq!(out.verdict, Some(Verdict::Require));
        assert_eq!(out.artifacts.len(), 1);
        assert_eq!(out.artifacts[0].mime, "text/html");

        // Should have QueueConsentTicket + WriteStorage
        assert_eq!(out.effects.len(), 2);
        match &out.effects[0] {
            Effect::QueueConsentTicket { k, n, required_roles, .. } => {
                assert_eq!(*k, 2);
                assert_eq!(*n, 3);
                assert_eq!(required_roles.len(), 3);
            }
            _ => panic!("expected QueueConsentTicket"),
        }
    }
}
