pub mod config;
pub mod api;
pub mod engine;

use anyhow::Result;
use std::sync::OnceLock;

pub use config::PricingConfig;

static CONF: OnceLock<PricingConfig> = OnceLock::new();

/// Load pricing config from a YAML file. Call once at startup.
pub fn load_pricing_from(path: impl AsRef<std::path::Path>) -> Result<()> {
    let text = std::fs::read_to_string(path)?;
    let cfg: PricingConfig = serde_yaml::from_str(&text)?;
    let _ = CONF.set(cfg);
    Ok(())
}

/// Get the loaded pricing config, if any.
pub fn get_config() -> Option<&'static PricingConfig> {
    CONF.get()
}

/// Price a single item using the loaded config.
pub fn price_one(req: &api::PriceReq) -> Result<api::PriceResp> {
    let cfg = CONF
        .get()
        .ok_or_else(|| anyhow::anyhow!("pricing config not loaded"))?;
    engine::price_one(cfg, req)
}

/// Price a scenario (multiple items) using the loaded config.
pub fn price_scenario(req: &api::ScenarioReq) -> Result<api::ScenarioResp> {
    let cfg = CONF
        .get()
        .ok_or_else(|| anyhow::anyhow!("pricing config not loaded"))?;
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
