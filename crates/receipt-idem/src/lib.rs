//! Helper para construir idempotency-key para AppendReceipt e payloads associados.
use anyhow::Result;

pub fn idempotency_key(tenant: &str, trace_id: &str, plan_cid: &str) -> String {
    let s = format!("{}:{}:{}", tenant, trace_id, plan_cid);
    let h = blake3::hash(s.as_bytes());
    format!("idem-{}", hex::encode(h.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn stable() {
        let k = idempotency_key("t","tr","pcid");
        assert!(k.starts_with("idem-"));
    }
}
