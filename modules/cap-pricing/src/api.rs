use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use indexmap::IndexMap;

#[derive(Debug, Clone, Deserialize)]
pub struct PriceReq {
    pub sku: String,
    pub qty: Option<Decimal>,
    /// Region code: BR, BR-SP, US-CA, etc.
    pub region: Option<String>,
    /// For rule matching
    pub category: Option<String>,
    /// Customer tier: "pro", "enterprise", etc.
    pub customer_tier: Option<String>,
    /// Applicable coupons
    pub coupons: Option<Vec<String>>,
    /// Include detailed step breakdown
    #[serde(default)]
    pub explain: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PriceResp {
    pub sku: String,
    pub unit_list: Decimal,
    pub unit_net: Decimal,
    pub unit_tax: Decimal,
    pub unit_total: Decimal,
    pub qty: Decimal,
    pub total_net: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub kind: String,
    pub name: String,
    pub before: Decimal,
    pub after: Decimal,
    pub meta: IndexMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScenarioReq {
    pub items: Vec<PriceReq>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioResp {
    pub items: Vec<PriceResp>,
    pub grand_total: Decimal,
}
