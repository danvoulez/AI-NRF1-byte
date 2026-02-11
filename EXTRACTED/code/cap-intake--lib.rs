use anyhow::Context;
use modules_core::{Capability, CapInput, CapOutput};
use serde::Deserialize;
use serde_json::{json, Value};

/// Regra de mapeamento: copia do `from` (dot-path) para `to` (dot-path) no env.
#[derive(Debug, Deserialize)]
struct MapRule {
    from: String, // ex: "req.body.user.id"
    to: String,   // ex: "ctx.user.id"
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    mode: Option<String>, // "document"|"event"|"transaction" (futuro)
    #[serde(default)]
    mapping: Vec<MapRule>,
    #[serde(default)]
    defaults: serde_json::Map<String, Value>, // chaves/valores iniciais no destino (facilita)
}

#[derive(Default)]
pub struct IntakeModule;

impl IntakeModule {
    fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
        let mut cur = root;
        for seg in path.split('.') {
            match cur {
                Value::Object(m) => { cur = m.get(seg)?; }
                Value::Array(a) => {
                    let idx: usize = seg.parse().ok()?;
                    cur = a.get(idx)?;
                }
                _ => return None,
            }
        }
        Some(cur)
    }

    fn ensure_object_path<'a>(root: &'a mut Value, path: &str) -> anyhow::Result<&'a mut Value> {
        let mut cur = root;
        for seg in path.split('.') {
            match cur {
                Value::Object(m) => {
                    if !m.contains_key(seg) { m.insert(seg.to_string(), json!({})); }
                    cur = m.get_mut(seg).unwrap();
                }
                _ => anyhow::bail!("path collision at '{}'", seg),
            }
        }
        Ok(cur)
    }

    fn set(root: &mut Value, path: &str, val: Value) -> anyhow::Result<()> {
        if let Some((parent, leaf)) = path.rsplit_once('.') {
            let obj = Self::ensure_object_path(root, parent)?;
            match obj {
                Value::Object(m) => { m.insert(leaf.to_string(), val); Ok(()) }
                _ => anyhow::bail!("non-object at parent path '{}'", parent),
            }
        } else {
            // raiz
            *root = val;
            Ok(())
        }
    }

    fn apply_defaults(dst: &mut Value, defaults: &serde_json::Map<String, Value>) -> anyhow::Result<()> {
        for (k, v) in defaults {
            if dst.get(k).is_none() {
                Self::set(dst, k, v.clone())?;
            }
        }
        Ok(())
    }

    fn transform(&self, env: &ai_nrf1::Value, cfg: &Config) -> anyhow::Result<ai_nrf1::Value> {
        // 1) Converte env NRF → JSON (para trabalhar com paths)
        let mut j = ubl_json_view::to_json(env).context("to_json(view) failed")?;

        // 2) Aplica defaults no destino
        if !cfg.defaults.is_empty() {
            Self::apply_defaults(&mut j, &cfg.defaults)?;
        }

        // 3) Executa mapeamentos
        for rule in &cfg.mapping {
            let val = Self::get(&j, &rule.from)
                .cloned()
                .unwrap_or(Value::Null); // fontes ausentes viram Null (a policy vai decidir)
            Self::set(&mut j, &rule.to, val)?;
        }

        // 4) Volta JSON → NRF (validação canônica ocorre aqui)
        let out = ubl_json_view::from_json(&j).context("from_json(view) failed")?;
        Ok(out)
    }
}

impl Capability for IntakeModule {
    const KIND: &'static str = "cap-intake";
    const API_VERSION: &'static str = "1.1.0";

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let _cfg: Config = serde_json::from_value(config.clone())
            .context("invalid cap-intake config")?;
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let new_env = self.transform(&input.env, &cfg)?;
        Ok(CapOutput { new_env: Some(new_env), ..Default::default() })
    }
}
