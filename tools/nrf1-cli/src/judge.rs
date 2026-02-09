
use nrf_core::{Value, encode};
use std::time::{SystemTime, UNIX_EPOCH};

/// Stub judge: produces a canonical NRF value representing a judgment.
/// In production this would create a full `receipt::Receipt`.
pub fn run_judge(context_bytes: &[u8], policy_ref: &str, allow: bool) -> Value {
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as i64;
    let inputs_cid = format!("b3:{}", blake3::hash(context_bytes).to_hex());
    let decision = if allow { "ALLOW" } else { "GHOST" };

    let mut m = std::collections::BTreeMap::new();
    m.insert("v".into(), Value::String("receipt-v1".into()));
    m.insert("t".into(), Value::Int(t));
    m.insert("act".into(), Value::String("EVALUATE".into()));
    m.insert("decision".into(), Value::String(decision.into()));
    m.insert("issuer_did".into(), Value::String("did:ubl:demo".into()));
    m.insert("inputs_cid".into(), Value::String(inputs_cid));
    m.insert("policy".into(), Value::String(policy_ref.into()));
    Value::Map(m)
}
