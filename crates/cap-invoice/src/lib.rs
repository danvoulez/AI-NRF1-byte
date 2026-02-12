pub mod types;
pub mod store;

use types::*;
use store::FsStore;
use anyhow::{anyhow, Result};
use uuid::Uuid;
use std::sync::OnceLock;

static STORE: OnceLock<FsStore> = OnceLock::new();

/// Initialize the invoice store. Call once at startup.
pub fn init_store() {
    let _ = STORE.set(FsStore::from_env());
}

fn get_store() -> Result<&'static FsStore> {
    STORE.get().ok_or_else(|| anyhow!("invoice store not initialized"))
}

/// Request to create an invoice from a quote.
#[derive(serde::Deserialize)]
pub struct CreateFromQuoteReq {
    pub quote_id: Uuid,
    pub customer: CustomerRef,
    pub currency: Option<String>,
    pub require_ack: Option<bool>,
}

/// Create an invoice stub from an existing quote.
pub fn create_from_quote(req: &CreateFromQuoteReq) -> Result<InvoiceStub> {
    let store = get_store()?;
    let q = cap_quote::get_quote_by_id(req.quote_id)
        .ok_or_else(|| anyhow!("quote not found: {}", req.quote_id))?;

    let id = Uuid::new_v4();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let issued_at = format!("{}Z", now); // simplified ISO timestamp

    let lines = q
        .items
        .iter()
        .map(|l| InvoiceLine {
            sku: l.sku.clone(),
            qty: l.qty,
            unit_total: l.unit_total,
            line_total: l.line_total,
        })
        .collect::<Vec<_>>();

    let stub = InvoiceStub {
        id,
        quote_id: q.id,
        customer: req.customer.clone(),
        lines,
        totals: Totals {
            subtotal: q.totals.subtotal,
            tax: q.totals.tax,
            total: q.totals.total,
        },
        currency: req.currency.clone().unwrap_or_else(|| "USD".into()),
        issued_at,
        status: if req.require_ack.unwrap_or(true) {
            InvoiceStatus::PendingAsk
        } else {
            InvoiceStatus::Approved
        },
    };

    let key = format!("invoices/{id}.json");
    store.put_json(&key, &stub)?;
    tracing::info!(%id, "invoice created");
    Ok(stub)
}

/// Get an invoice by ID.
pub fn get_invoice(id: Uuid) -> Result<Option<InvoiceStub>> {
    let store = get_store()?;
    let key = format!("invoices/{id}.json");
    store.get_json(&key)
}

/// Approve an invoice (PENDING_ASK → APPROVED).
pub fn approve_invoice(id: Uuid) -> Result<InvoiceStub> {
    let store = get_store()?;
    let key = format!("invoices/{id}.json");
    let mut inv: InvoiceStub = store
        .get_json(&key)?
        .ok_or_else(|| anyhow!("invoice not found: {id}"))?;
    inv.status = InvoiceStatus::Approved;
    store.put_json(&key, &inv)?;
    tracing::info!(%id, "invoice approved");
    Ok(inv)
}

/// Reject an invoice (PENDING_ASK → REJECTED).
pub fn reject_invoice(id: Uuid) -> Result<InvoiceStub> {
    let store = get_store()?;
    let key = format!("invoices/{id}.json");
    let mut inv: InvoiceStub = store
        .get_json(&key)?
        .ok_or_else(|| anyhow!("invoice not found: {id}"))?;
    inv.status = InvoiceStatus::Rejected;
    store.put_json(&key, &inv)?;
    tracing::info!(%id, "invoice rejected");
    Ok(inv)
}
