use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceStub {
    pub id: Uuid,
    pub quote_id: Uuid,
    pub customer: CustomerRef,
    pub lines: Vec<InvoiceLine>,
    pub totals: Totals,
    /// "USD", "BRL", etc.
    pub currency: String,
    /// RFC3339
    pub issued_at: String,
    pub status: InvoiceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerRef {
    pub id: String,
    pub name: Option<String>,
    pub tax_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub sku: String,
    pub qty: Decimal,
    pub unit_total: Decimal,
    pub line_total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Totals {
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceStatus {
    Draft,
    PendingAsk,
    Approved,
    Rejected,
}

impl Default for InvoiceStatus {
    fn default() -> Self {
        Self::Draft
    }
}
