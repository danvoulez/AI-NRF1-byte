//! cap-pricing — pure pricing evaluation capability.
//!
//! Implements `Capability` trait: receives pricing config via `input.config`,
//! item data via `input.env`, returns priced result in `new_env`.
//! No global state. No file IO. Pure evaluation.

pub mod config;
pub mod api;
pub mod engine;

use anyhow::{Context, Result};
use modules_core::{CapInput, CapOutput, Capability};
use serde_json::Value as JsonValue;

pub use config::PricingConfig;

// ---------------------------------------------------------------------------
// Capability implementation (constitutional: pure, no IO, no global state)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct PricingModule;

impl Capability for PricingModule {
    fn kind(&self) -> &'static str { "cap-pricing" }
    fn api_version(&self) -> &'static str { "1.0" }

    fn validate_config(&self, cfg: &serde_json::Value) -> Result<()> {
        let _: PricingConfig = serde_json::from_value(cfg.clone())
            .context("cap-pricing: invalid pricing config in manifest")?;
        Ok(())
    }

    fn execute(&self, input: CapInput) -> Result<CapOutput> {
        let cfg: PricingConfig = serde_json::from_value(input.config.clone())
            .context("cap-pricing: invalid pricing config")?;

        let env_json: JsonValue = ubl_json_view::to_json(&input.env);

        // Extract pricing request from env (set by cap-intake)
        let req: api::PriceReq = serde_json::from_value(env_json.clone())
            .context("cap-pricing: env does not contain a valid PriceReq")?;

        let result = engine::price_one(&cfg, &req)?;
        let result_json = serde_json::to_value(&result)?;

        // Merge result into env under "pricing" key
        let mut out_env = env_json.clone();
        if let Some(obj) = out_env.as_object_mut() {
            obj.insert("pricing".into(), result_json);
        }

        let new_env = ubl_json_view::from_json(&out_env)
            .context("cap-pricing: failed to convert result back to NRF Value")?;

        Ok(CapOutput {
            new_env: Some(new_env),
            verdict: None,
            artifacts: vec![],
            effects: vec![],
            metrics: vec![
                ("pricing.unit_total_cents".into(),
                 (result.unit_total * rust_decimal::Decimal::from(100))
                     .to_string().parse::<i64>().unwrap_or(0)),
            ],
        })
    }
}

// ---------------------------------------------------------------------------
// Service-layer convenience functions (used by crates/cap-quote, registry).
// These take an explicit config — no global state.
// The old global OnceLock pattern is removed (constitutional violation).
// Callers must manage their own config lifecycle.
// ---------------------------------------------------------------------------

/// Price a single item with an explicit config. Pure.
pub fn price_one_with(cfg: &PricingConfig, req: &api::PriceReq) -> Result<api::PriceResp> {
    engine::price_one(cfg, req)
}

/// Price a scenario (multiple items) with an explicit config. Pure.
pub fn price_scenario_with(cfg: &PricingConfig, req: &api::ScenarioReq) -> Result<api::ScenarioResp> {
    let mut items = vec![];
    let mut grand = rust_decimal::Decimal::ZERO;
    for it in &req.items {
        let r = engine::price_one(cfg, it)?;
        grand += r.total;
        items.push(r);
    }
    Ok(api::ScenarioResp {
        items,
        grand_total: grand,
    })
}

// ---------------------------------------------------------------------------
// Backward-compat: global config for service-layer callers that need it.
// This is NOT used by the Capability::execute() path.
// ---------------------------------------------------------------------------

use std::sync::OnceLock;
static CONF: OnceLock<PricingConfig> = OnceLock::new();

/// Load pricing config from a YAML string. Service-layer use only.
pub fn load_pricing_from(path: impl AsRef<std::path::Path>) -> Result<()> {
    let text = std::fs::read_to_string(path)?;
    let cfg: PricingConfig = serde_yaml::from_str(&text)?;
    let _ = CONF.set(cfg);
    Ok(())
}

/// Get the loaded pricing config. Service-layer use only.
pub fn get_config() -> Option<&'static PricingConfig> {
    CONF.get()
}

/// Price using global config. Service-layer convenience only.
pub fn price_one(req: &api::PriceReq) -> Result<api::PriceResp> {
    let cfg = CONF.get().ok_or_else(|| anyhow::anyhow!("pricing config not loaded"))?;
    engine::price_one(cfg, req)
}
