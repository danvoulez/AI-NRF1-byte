use anyhow::Context;
use modules_core::{Capability, CapInput, CapOutput, Verdict};
use serde::Deserialize;
use serde_json::Value;
use ubl_json_view;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
enum Rule {
    /// Campos devem existir e não ser Null.
    Exist { paths: Vec<String> },

    /// Inteiro escalado no path deve ser >= min (ambos i64).
    Threshold { path: String, min: i64 },

    /// Inteiro escalado no path deve estar no intervalo [min, max].
    ThresholdRange { path: String, min: i64, max: i64 },

    /// Valor (string/i64) no path deve estar numa lista.
    Allowlist { path: String, values: Vec<Value> },

    /// Nega uma sub-regra (NOT).
    Not { rule: Box<Rule> },
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    rules: Vec<Rule>,
    /// "DENY" (padrão) ou "REQUIRE" quando alguma regra falha
    #[serde(default)]
    decision_on_fail: Option<String>,
}

#[derive(Default)]
pub struct PolicyModule;

impl PolicyModule {
    fn get<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
        let mut cur = root;
        for seg in path.split('.') {
            match cur {
                Value::Object(m) => { cur = m.get(seg)?; }
                Value::Array(a) => { cur = a.get(seg.parse::<usize>().ok()?)?; }
                _ => return None,
            }
        }
        Some(cur)
    }

    fn as_i64(v: &Value) -> Option<i64> {
        match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.parse::<i64>().ok(),
            _ => None,
        }
    }

    fn rule_ok(j: &Value, r: &Rule) -> bool {
        match r {
            Rule::Exist { paths } => {
                paths.iter().all(|p| Self::get(j, p).is_some() && !matches!(Self::get(j, p), Some(Value::Null)))
            }
            Rule::Threshold { path, min } => {
                Self::get(j, path).and_then(Self::as_i64).map(|v| v >= *min).unwrap_or(false)
            }
            Rule::ThresholdRange { path, min, max } => {
                Self::get(j, path).and_then(Self::as_i64).map(|v| v >= *min && v <= *max).unwrap_or(false)
            }
            Rule::Allowlist { path, values } => {
                match Self::get(j, path) {
                    Some(v) => values.iter().any(|x| x == v),
                    None => false,
                }
            }
            Rule::Not { rule } => !Self::rule_ok(j, rule),
        }
    }
}

impl Capability for PolicyModule {
    const KIND: &'static str = "cap-policy";
    const API_VERSION: &'static str = "1.2.0";

    fn validate_config(&self, config: &Value) -> anyhow::Result<()> {
        let _cfg: Config = serde_json::from_value(config.clone())
            .context("invalid cap-policy config")?;
        Ok(())
    }

    fn execute(&self, input: CapInput) -> anyhow::Result<CapOutput> {
        // NRF → JSON view
        let j = ubl_json_view::to_json(&input.env)?;

        let cfg: Config = serde_json::from_value(input.config.clone())?;
        let fail_verdict = match cfg.decision_on_fail.as_deref() {
            Some("REQUIRE") => Verdict::Require,
            _ => Verdict::Deny,
        };

        let mut failed = 0usize;
        for r in &cfg.rules {
            if !Self::rule_ok(&j, r) { failed += 1; }
        }

        let verdict = if failed == 0 { Verdict::Allow } else { fail_verdict };

        Ok(CapOutput {
            verdict: Some(verdict),
            metrics: vec![("rules_failed".into(), failed as i64)],
            ..Default::default()
        })
    }
}
