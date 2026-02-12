pub mod api;

use api::*;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use rust_decimal::Decimal;
use std::sync::LazyLock;
use uuid::Uuid;

static QUOTES: LazyLock<DashMap<Uuid, QuoteResp>> = LazyLock::new(DashMap::new);

/// Create a quote from a list of items, using the loaded pricing config.
pub fn create_quote(req: &QuoteCreateReq) -> Result<QuoteResp> {
    let cfg = cap_pricing::get_config()
        .ok_or_else(|| anyhow!("pricing config not loaded"))?;

    let id = Uuid::new_v4();
    let mut lines = vec![];
    let mut subtotal = Decimal::ZERO;
    let mut tax = Decimal::ZERO;
    let mut total = Decimal::ZERO;

    for it in &req.items {
        let pr = cap_pricing::api::PriceReq {
            sku: it.sku.clone(),
            qty: it.qty.or(Some(Decimal::ONE)),
            region: it.region.clone().or(req.region.clone()),
            category: it.category.clone(),
            customer_tier: req.customer_tier.clone(),
            coupons: it.coupons.clone().or(req.coupons.clone()),
            explain: false,
        };
        let priced = cap_pricing::engine::price_one(cfg, &pr)?;
        subtotal += priced.total_net;
        tax += priced.total_tax;
        total += priced.total;

        lines.push(QuoteLine {
            sku: pr.sku,
            qty: pr.qty.unwrap_or(Decimal::ONE),
            unit_net: priced.unit_net,
            unit_tax: priced.unit_tax,
            unit_total: priced.unit_total,
            line_total: priced.total,
        });
    }

    let resp = QuoteResp {
        id,
        items: lines,
        totals: QuoteTotals {
            subtotal,
            tax,
            total,
        },
    };
    QUOTES.insert(id, resp.clone());
    Ok(resp)
}

/// Retrieve a quote by ID.
pub fn get_quote_by_id(id: Uuid) -> Option<QuoteResp> {
    QUOTES.get(&id).map(|x| x.clone())
}

/// Reprice a quote (re-read from store, return as-is for now).
pub fn reprice_quote(id: Uuid) -> Option<QuoteResp> {
    QUOTES.get(&id).map(|x| x.clone())
}
