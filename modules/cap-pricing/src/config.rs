use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use indexmap::IndexMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    /// Base prices by SKU (list price)
    pub list: IndexMap<String, Decimal>,

    /// Rules: ordered, first match applies (multiple if stackable=true)
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// Tax config by region (default + overrides)
    #[serde(default)]
    pub tax: TaxConfig,

    /// Rounding config
    #[serde(default = "default_round")]
    pub rounding: Rounding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    /// Target type: "sku" | "category" | "customer_tier" | "coupon"
    pub target: String,
    /// Exact value or simple glob (e.g. "PLAN-*")
    pub matcher: String,
    /// Action: "discount_pct" | "discount_abs" | "surcharge_pct" | "surcharge_abs"
    pub action: String,
    pub value: Decimal,
    /// Stack with subsequent matching rules? (default: false)
    #[serde(default)]
    pub stackable: bool,
    /// Priority (lower first). If absent, file order applies.
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaxConfig {
    pub default_pct: Option<Decimal>,
    /// Overrides by region code (e.g. "BR-SP": 0.19)
    #[serde(default)]
    pub by_region: IndexMap<String, Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rounding {
    /// Decimal places (typically 2)
    pub scale: u32,
    /// "bankers" | "half_up"
    pub mode: String,
}

fn default_round() -> Rounding {
    Rounding {
        scale: 2,
        mode: "half_up".into(),
    }
}
