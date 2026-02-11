//! Binding resolver: resolves `env:VAR` references from io_bindings at runtime.
//! Never logs resolved secret values.

use serde_json::Value;

/// Resolve a binding value from the product manifest's `io_bindings`.
/// If the value starts with `env:`, read from environment variable.
/// Otherwise return the literal string.
pub fn resolve(io_bindings: &Value, key: &str) -> anyhow::Result<String> {
    let raw = io_bindings
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("binding not found: {}", key))?;

    if let Some(var_name) = raw.strip_prefix("env:") {
        std::env::var(var_name)
            .map_err(|_| anyhow::anyhow!("env var not set: {} (binding: {})", var_name, key))
    } else {
        Ok(raw.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolve_literal() {
        let bindings = json!({"webhook.url": "https://example.com"});
        assert_eq!(
            resolve(&bindings, "webhook.url").unwrap(),
            "https://example.com"
        );
    }

    #[test]
    fn resolve_missing() {
        let bindings = json!({});
        assert!(resolve(&bindings, "missing").is_err());
    }

    #[test]
    fn resolve_env_var() {
        std::env::set_var("__TEST_BINDING_VAR", "secret123");
        let bindings = json!({"key": "env:__TEST_BINDING_VAR"});
        assert_eq!(resolve(&bindings, "key").unwrap(), "secret123");
        std::env::remove_var("__TEST_BINDING_VAR");
    }

    #[test]
    fn resolve_env_var_missing() {
        let bindings = json!({"key": "env:__NONEXISTENT_VAR_12345"});
        assert!(resolve(&bindings, "key").is_err());
    }
}
