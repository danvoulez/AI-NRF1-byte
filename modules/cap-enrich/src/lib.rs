//! cap-enrich: render receipts/decisions into artifacts (HTML status page, webhook).
//! See design doc §9D: "cap-enrich (Render)".
//!
//! Drivers are declarative — the module returns `Artifact` blobs and `Effect`
//! requests; the runtime's `EffectExecutor` handles actual IO.

use modules_core::{Artifact, CapInput, CapOutput, Capability, Effect};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum DriverKind {
    StatusPage,
    Webhook,
}

#[derive(Debug, Deserialize)]
struct Driver {
    kind: DriverKind,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    drivers: Vec<Driver>,
    #[serde(default)]
    redaction: Vec<String>,
    #[serde(default)]
    webhook_binding: Option<String>,
}

#[derive(Default)]
pub struct EnrichModule;

impl EnrichModule {
    fn redact(mut j: Value, paths: &[String]) -> Value {
        for p in paths {
            if let Some((parent, leaf)) = p.rsplit_once('.') {
                if let Some(obj) = Self::get_mut(&mut j, parent) {
                    if let Some(m) = obj.as_object_mut() {
                        m.insert(leaf.to_string(), json!("<redacted>"));
                    }
                }
            }
        }
        j
    }

    fn get_mut<'a>(root: &'a mut Value, path: &str) -> Option<&'a mut Value> {
        let mut cur = root;
        for seg in path.split('.') {
            match cur {
                Value::Object(m) => {
                    cur = m.get_mut(seg)?;
                }
                Value::Array(a) => {
                    cur = a.get_mut(seg.parse::<usize>().ok()?)?;
                }
                _ => return None,
            }
        }
        Some(cur)
    }

    fn html_status(name: &str, redacted_json: &Value) -> Artifact {
        let pretty = serde_json::to_string_pretty(redacted_json).unwrap_or_else(|_| "{}".into());
        let html = format!(
            r#"<!doctype html>
<html>
<head><meta charset="utf-8"><title>Status — {name}</title></head>
<body>
  <h1>Status</h1>
  <pre>{escaped}</pre>
</body>
</html>"#,
            name = name,
            escaped = html_escape(&pretty),
        );
        Artifact {
            cid: None,
            mime: "text/html".into(),
            bytes: html.into_bytes(),
            name: Some("status.html".into()),
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Capability for EnrichModule {
    fn kind(&self) -> &'static str {
        "cap-enrich"
    }
    fn api_version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let _cfg: Config = serde_json::from_value(config.clone())?;
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let mut artifacts = vec![];
        let mut effects = vec![];

        let j = ubl_json_view::to_json(&input.env);
        let j = Self::redact(j, &cfg.redaction);

        for d in &cfg.drivers {
            match d.kind {
                DriverKind::StatusPage => {
                    let art = Self::html_status("capsule", &j);
                    effects.push(Effect::WriteStorage {
                        path: "status.html".into(),
                        bytes: art.bytes.clone(),
                        mime: "text/html".into(),
                    });
                    artifacts.push(art);
                }
                DriverKind::Webhook => {
                    let body = serde_json::to_vec(&j).unwrap_or_default();
                    effects.push(Effect::Webhook {
                        url: "<binding:webhook.url>".into(),
                        body,
                        content_type: "application/json".into(),
                        hmac_key_env: cfg.webhook_binding.clone(),
                    });
                }
            }
        }

        Ok(CapOutput {
            artifacts,
            effects,
            ..Default::default()
        })
    }
}
