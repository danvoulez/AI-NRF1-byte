use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct QuoteCreateReq {
    pub items: Vec<QuoteItemReq>,
    /// Default region for items without one
    pub region: Option<String>,
    pub customer_tier: Option<String>,
    pub coupons: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuoteItemReq {
    pub sku: String,
    pub qty: Option<Decimal>,
    pub region: Option<String>,
    pub category: Option<String>,
    pub coupons: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteResp {
    pub id: Uuid,
    pub items: Vec<QuoteLine>,
    pub totals: QuoteTotals,
}

#[derive(Debug, Clone, Serialize)]
pub struct QuoteLine {
    pub sku: String,
    pub qty: Decimal,
    pub unit_net: Decimal,
    pub unit_tax: Decimal,
    pub unit_total: Decimal,
    pub line_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct QuoteTotals {
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuoteRepriceReq {
    pub id: Uuid,
}
