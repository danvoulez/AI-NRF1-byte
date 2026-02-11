use modules_core::{Capability, CapInput, CapOutput, Artifact, Effect};
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
    #[serde(default)]
    template_cid: Option<String>, // futuro: templates versionados por CID
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    drivers: Vec<Driver>,
    #[serde(default)]
    redaction: Vec<String>, // dot-paths para ocultar na view
    #[serde(default)]
    webhook_binding: Option<String>, // nome do binding cujo segredo assina HMAC (WH_SEC)
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
                Value::Object(m) => { cur = m.get_mut(seg)?; }
                Value::Array(a) => { cur = a.get_mut(seg.parse::<usize>().ok()?)?; }
                _ => return None,
            }
        }
        Some(cur)
    }

    fn html_status(name: &str, redacted_json: &Value) -> Artifact {
        let pretty = serde_json::to_string_pretty(redacted_json).unwrap_or_else(|_| "{}".into());
        let html = format!(r#"<!doctype html>
<html>
<head><meta charset="utf-8"><title>Status — {}</title></head>
<body>
  <h1>Status</h1>
  <pre>{}</pre>
</body>
</html>"#, name, htmlescape::encode_minimal(&pretty));
        Artifact::Html { name: "status.html".into(), html }
    }
}

impl Capability for EnrichModule {
    const KIND: &'static str = "cap-enrich";
    const API_VERSION: &'static str = "1.0.0";

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let _cfg: Config = serde_json::from_value(config.clone())?;
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let mut artifacts = vec![];
        let mut effects = vec![];

        // env NRF → JSON (para redaction/visual)
        let mut j = ubl_json_view::to_json(&input.env)?;
        j = Self::redact(j, &cfg.redaction);

        for d in &cfg.drivers {
            match d.kind {
                DriverKind::StatusPage => {
                    artifacts.push(Self::html_status("capsule", &j));
                    // também solicitar a publicação (effect) para o executor
                    if let Artifact::Html { html, .. } = artifacts.last().unwrap().clone() {
                        effects.push(Effect::PublishStatusPage { name: "status.html".into(), html });
                    }
                }
                DriverKind::Webhook => {
                    effects.push(Effect::Webhook {
                        url: "<binding:webhook.url>".into(), // o executor substitui via io_binding
                        headers: vec![("content-type".into(), "application/json".into())],
                        body: j.clone(),
                        hmac_binding: cfg.webhook_binding.clone(), // ex.: Some("WH_SEC")
                    });
                }
            }
        }

        Ok(CapOutput { artifacts, effects, ..Default::default() })
    }
}
