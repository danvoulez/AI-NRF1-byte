//! cap-intake: normalize raw input into canonical env via declarative mapping DSL.
//! See design doc §9A: "cap-intake (Normalize)".

use anyhow::Context;
use modules_core::{CapInput, CapOutput, Capability};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct MapRule {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    mapping: Vec<MapRule>,
    #[serde(default)]
    defaults: serde_json::Map<String, Value>,
}

#[derive(Default)]
pub struct IntakeModule;

impl IntakeModule {
    fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
        let mut cur = root;
        for seg in path.split('.') {
            match cur {
                Value::Object(m) => {
                    cur = m.get(seg)?;
                }
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
                    if !m.contains_key(seg) {
                        m.insert(seg.to_string(), json!({}));
                    }
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
                Value::Object(m) => {
                    m.insert(leaf.to_string(), val);
                    Ok(())
                }
                _ => anyhow::bail!("non-object at parent path '{}'", parent),
            }
        } else {
            // Single-segment path: insert as key in root object (don't replace root)
            match root {
                Value::Object(m) => {
                    m.insert(path.to_string(), val);
                    Ok(())
                }
                _ => {
                    // Root is not an object — wrap it
                    let mut m = serde_json::Map::new();
                    m.insert(path.to_string(), val);
                    *root = Value::Object(m);
                    Ok(())
                }
            }
        }
    }

    fn apply_defaults(
        dst: &mut Value,
        defaults: &serde_json::Map<String, Value>,
    ) -> anyhow::Result<()> {
        for (k, v) in defaults {
            if dst.get(k).is_none() {
                Self::set(dst, k, v.clone())?;
            }
        }
        Ok(())
    }

    fn transform(&self, env: &nrf1::Value, cfg: &Config) -> anyhow::Result<nrf1::Value> {
        let mut j = ubl_json_view::to_json(env);

        if !cfg.defaults.is_empty() {
            Self::apply_defaults(&mut j, &cfg.defaults)?;
        }

        for rule in &cfg.mapping {
            let val = Self::get(&j, &rule.from)
                .cloned()
                .unwrap_or(Value::Null);
            Self::set(&mut j, &rule.to, val)?;
        }

        let out = ubl_json_view::from_json(&j).context("from_json(view) failed")?;
        Ok(out)
    }
}

impl Capability for IntakeModule {
    fn kind(&self) -> &'static str {
        "cap-intake"
    }
    fn api_version(&self) -> &'static str {
        "1.1.0"
    }

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let _cfg: Config =
            serde_json::from_value(config.clone()).context("invalid cap-intake config")?;
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let new_env = self.transform(&input.env, &cfg)?;
        Ok(CapOutput {
            new_env: Some(new_env),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modules_core::{AssetResolver, ExecutionMeta};
    use std::collections::BTreeMap;

    #[derive(Clone)]
    struct NullResolver;
    impl AssetResolver for NullResolver {
        fn get(&self, _cid: &modules_core::Cid) -> anyhow::Result<modules_core::Asset> {
            anyhow::bail!("no assets")
        }
        fn box_clone(&self) -> Box<dyn AssetResolver> {
            Box::new(self.clone())
        }
    }

    fn make_input(env: nrf1::Value, config: Value) -> modules_core::CapInput {
        modules_core::CapInput {
            env,
            config,
            assets: Box::new(NullResolver),
            prev_receipts: vec![],
            meta: ExecutionMeta {
                run_id: "test".into(),
                tenant: None,
                trace_id: None,
                ts_nanos: 0,
            },
        }
    }

    fn nrf_map(pairs: &[(&str, &str)]) -> nrf1::Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), nrf1::Value::String(v.to_string()));
        }
        nrf1::Value::Map(m)
    }

    // Regression: single-segment set inserts key, doesn't replace root
    #[test]
    fn set_single_key_inserts_into_root() {
        let env = nrf_map(&[("data", "hello")]);
        let cfg = json!({ "mapping": [{ "from": "data", "to": "payload" }] });
        let input = make_input(env, cfg);
        let out = IntakeModule.execute(input).unwrap();
        let j = ubl_json_view::to_json(out.new_env.as_ref().unwrap());
        assert_eq!(j["payload"], "hello");
        assert_eq!(j["data"], "hello"); // original key preserved
    }

    // Nested path creates intermediate objects
    #[test]
    fn set_nested_creates_intermediates() {
        let env = nrf_map(&[("x", "1")]);
        let cfg = json!({ "mapping": [{ "from": "x", "to": "a.b.c" }] });
        let input = make_input(env, cfg);
        let out = IntakeModule.execute(input).unwrap();
        let j = ubl_json_view::to_json(out.new_env.as_ref().unwrap());
        assert_eq!(j["a"]["b"]["c"], "1");
    }

    // Defaults fill missing keys
    #[test]
    fn defaults_fill_missing() {
        let env = nrf1::Value::Map(BTreeMap::new());
        let cfg = json!({ "defaults": { "status": "pending" } });
        let input = make_input(env, cfg);
        let out = IntakeModule.execute(input).unwrap();
        let j = ubl_json_view::to_json(out.new_env.as_ref().unwrap());
        assert_eq!(j["status"], "pending");
    }

    // Mapping from missing key produces null
    #[test]
    fn missing_from_produces_null() {
        let env = nrf_map(&[("a", "1")]);
        let cfg = json!({ "mapping": [{ "from": "nonexistent", "to": "dest" }] });
        let input = make_input(env, cfg);
        let out = IntakeModule.execute(input).unwrap();
        let j = ubl_json_view::to_json(out.new_env.as_ref().unwrap());
        assert!(j["dest"].is_null());
    }
}
