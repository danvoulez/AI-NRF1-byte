use crate::api::{PriceReq, PriceResp, Step};
use crate::config::{PricingConfig, Rounding};
use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use rust_decimal::Decimal;

pub fn price_one(cfg: &PricingConfig, req: &PriceReq) -> Result<PriceResp> {
    let qty = req.qty.unwrap_or(Decimal::ONE);
    let list = cfg
        .list
        .get(&req.sku)
        .ok_or_else(|| anyhow!("unknown sku: {}", req.sku))?
        .clone();

    let mut steps = vec![];
    let mut unit = list;
    steps.push(step("list", "base", unit, unit, IndexMap::new()));

    // Apply rules sorted by priority
    let mut rules = cfg.rules.clone();
    rules.sort_by_key(|r| r.priority.unwrap_or(i32::MAX));

    for r in &rules {
        if !rule_matches(&r.target, &r.matcher, req) {
            continue;
        }
        let before = unit;
        unit = match r.action.as_str() {
            "discount_pct" => unit * (Decimal::ONE - r.value / dec(100)),
            "discount_abs" => unit - r.value,
            "surcharge_pct" => unit * (Decimal::ONE + r.value / dec(100)),
            "surcharge_abs" => unit + r.value,
            _ => before,
        };
        let mut meta = IndexMap::new();
        meta.insert("rule".into(), r.name.clone());
        meta.insert("action".into(), r.action.clone());
        meta.insert("value".into(), r.value.to_string());
        steps.push(step("rule", &r.name, before, unit, meta));

        if !r.stackable {
            break;
        }
    }

    let unit_net = round_by(unit, &cfg.rounding);

    // Tax
    let pct = tax_pct(cfg, req.region.as_deref());
    let unit_tax = round_by(unit_net * pct, &cfg.rounding);
    let unit_total = round_by(unit_net + unit_tax, &cfg.rounding);

    {
        let mut m = IndexMap::new();
        m.insert("pct".into(), pct.to_string());
        steps.push(step("tax", "vat/sales", unit_net, unit_total, m));
    }

    // Totals
    let total_net = round_by(unit_net * qty, &cfg.rounding);
    let total_tax = round_by(unit_tax * qty, &cfg.rounding);
    let total = round_by(unit_total * qty, &cfg.rounding);

    Ok(PriceResp {
        sku: req.sku.clone(),
        unit_list: list,
        unit_net,
        unit_tax,
        unit_total,
        qty,
        total_net,
        total_tax,
        total,
        steps,
    })
}

fn rule_matches(target: &str, matcher: &str, req: &PriceReq) -> bool {
    let pattern = format!("^{}$", matcher.replace('*', ".*"));
    let re = match regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(_) => return matcher == target_value(target, req).unwrap_or_default(),
    };
    match target {
        "sku" => re.is_match(&req.sku),
        "category" => req
            .category
            .as_deref()
            .map(|c| re.is_match(c))
            .unwrap_or(false),
        "customer_tier" => req
            .customer_tier
            .as_deref()
            .map(|t| re.is_match(t))
            .unwrap_or(false),
        "coupon" => req
            .coupons
            .as_ref()
            .map(|v| v.iter().any(|c| re.is_match(c)))
            .unwrap_or(false),
        _ => false,
    }
}

fn target_value<'a>(target: &str, req: &'a PriceReq) -> Option<&'a str> {
    match target {
        "sku" => Some(&req.sku),
        "category" => req.category.as_deref(),
        "customer_tier" => req.customer_tier.as_deref(),
        _ => None,
    }
}

fn step(
    kind: &str,
    name: &str,
    before: Decimal,
    after: Decimal,
    meta: IndexMap<String, String>,
) -> Step {
    Step {
        kind: kind.into(),
        name: name.into(),
        before,
        after,
        meta,
    }
}

fn tax_pct(cfg: &PricingConfig, region: Option<&str>) -> Decimal {
    if let Some(r) = region {
        if let Some(p) = cfg.tax.by_region.get(r) {
            return *p;
        }
        // Fallback by country (e.g. BR-SP → BR)
        if let Some((cc, _)) = r.split_once('-') {
            if let Some(p) = cfg.tax.by_region.get(cc) {
                return *p;
            }
        }
    }
    cfg.tax.default_pct.unwrap_or(dec(0))
}

fn round_by(v: Decimal, r: &Rounding) -> Decimal {
    use rust_decimal::RoundingStrategy;
    let mode = match r.mode.as_str() {
        "bankers" => RoundingStrategy::MidpointNearestEven,
        _ => RoundingStrategy::MidpointAwayFromZero,
    };
    v.round_dp_with_strategy(r.scale, mode)
}

fn dec(x: i64) -> Decimal {
    Decimal::from(x)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;
    use std::str::FromStr;

    fn test_config() -> PricingConfig {
        serde_yaml::from_str(
            r#"
rounding: { scale: 2, mode: half_up }
list:
  PLAN-PRO: 49.00
  PLAN-BASIC: 19.90
rules:
  - name: coupon-PLAN-20
    target: coupon
    matcher: PLAN20
    action: discount_pct
    value: 20
    stackable: true
    priority: 20
  - name: tier-enterprise-10
    target: customer_tier
    matcher: enterprise
    action: discount_pct
    value: 10
    stackable: true
    priority: 10
tax:
  default_pct: 0.00
  by_region:
    BR: 0.12
    BR-SP: 0.19
    US: 0.00
"#,
        )
        .unwrap()
    }

    #[test]
    fn basic_price_no_rules() {
        let cfg = test_config();
        let req = PriceReq {
            sku: "PLAN-BASIC".into(),
            qty: Some(Decimal::ONE),
            region: Some("US".into()),
            category: None,
            customer_tier: None,
            coupons: None,
            explain: false,
        };
        let r = price_one(&cfg, &req).unwrap();
        assert_eq!(r.unit_net, Decimal::from_str("19.90").unwrap());
        assert_eq!(r.unit_tax, Decimal::ZERO);
        assert_eq!(r.total, Decimal::from_str("19.90").unwrap());
    }

    #[test]
    fn coupon_with_tax() {
        let cfg = test_config();
        let req = PriceReq {
            sku: "PLAN-PRO".into(),
            qty: Some(Decimal::ONE),
            region: Some("BR".into()),
            category: None,
            customer_tier: None,
            coupons: Some(vec!["PLAN20".into()]),
            explain: false,
        };
        let r = price_one(&cfg, &req).unwrap();
        // 49 - 20% = 39.20; tax 12% of 39.20 = 4.704 → 4.70; total = 43.90
        assert_eq!(r.unit_net, Decimal::from_str("39.20").unwrap());
        assert_eq!(r.unit_tax, Decimal::from_str("4.70").unwrap());
        assert_eq!(r.unit_total, Decimal::from_str("43.90").unwrap());
    }

    #[test]
    fn unknown_sku_errors() {
        let cfg = test_config();
        let req = PriceReq {
            sku: "NOPE".into(),
            qty: None,
            region: None,
            category: None,
            customer_tier: None,
            coupons: None,
            explain: false,
        };
        assert!(price_one(&cfg, &req).is_err());
    }

    #[test]
    fn region_fallback() {
        let cfg = test_config();
        // BR-RJ not in by_region, falls back to BR (0.12)
        let req = PriceReq {
            sku: "PLAN-BASIC".into(),
            qty: Some(Decimal::ONE),
            region: Some("BR-RJ".into()),
            category: None,
            customer_tier: None,
            coupons: None,
            explain: false,
        };
        let r = price_one(&cfg, &req).unwrap();
        // 19.90 * 0.12 = 2.388 → 2.39
        assert_eq!(r.unit_tax, Decimal::from_str("2.39").unwrap());
    }
}
