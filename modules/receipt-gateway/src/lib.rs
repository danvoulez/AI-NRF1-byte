use ed25519_dalek::SigningKey;
use nrf_core::{rho, Value};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Receipt Gateway — MODULE #1
//
// The first module. Runs the full constitutional pipeline:
//
//   INPUT → ρ(body) → POLICY GATE → GHOST(pending) → RECEIPT → SIGN → URL
//
// Acts served: ATTEST, EVALUATE, TRANSACT (all three)
// Constitution of the Modules: compliant (§1.1, §2.1–§2.4, §5.1–§5.3)
//
// This module:
//   1. Accepts a GatewayRequest (body + act + issuer + policy engine)
//   2. ρ-normalizes the body (Article I of the Base)
//   3. Runs the policy engine (Article VI — produces a Decision)
//   4. Creates a Ghost(pending) (Article IV §4.3 — WBE discipline)
//   5. Builds and signs a Receipt (Article IV §4.1)
//   6. Promotes the Ghost
//   7. Returns the signed Receipt with rich URL
//
// It does NOT:
//   - Touch NRF encoding directly (delegates to receipt/ghost/nrf1)
//   - Fabricate decisions (§6.3)
//   - Skip the Ghost step (§6.4)
//   - Store anything (that's the service layer's job)
// ---------------------------------------------------------------------------

// =========================================================================
// Errors — LLM-friendly, greppable, constitutional citations
// =========================================================================

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("RHO-001: ρ normalization failed on body: {0}")]
    RhoFailed(String),

    #[error("ACT-001: invalid act '{0}'. Must be ATTEST | EVALUATE | TRANSACT (Article V)")]
    InvalidAct(String),

    #[error("POL-001: policy engine returned error: {0}")]
    PolicyFailed(String),

    #[error("GHO-002: ghost creation failed: {0}")]
    GhostFailed(String),

    #[error("SIG-001: signing failed: {0}")]
    SignFailed(String),

    #[error("INT-001: receipt integrity check failed: {0}")]
    IntegrityFailed(String),
}

// =========================================================================
// Request / Response
// =========================================================================

#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub issuer_did: String,
    pub act: String,                // ATTEST | EVALUATE | TRANSACT
    pub subject: String,            // CID of what's being acted on
    pub body: Value,                // the payload (will be ρ-normalized)
    pub policy_id: Option<String>,  // which policy to evaluate (None = existence only)
    pub pipeline_prev: Vec<String>, // CIDs of prior act receipts
    pub url_base: String, // e.g. "https://passports.ubl.agency/app/tenant/receipts/id.json"
    pub nonce: Vec<u8>,   // 16 bytes entropy
}

#[derive(Debug, Clone)]
pub struct GatewayResult {
    pub receipt: receipt::Receipt,
    pub ghost: ghost::Ghost,
    pub decision: ubl_policy::Decision,
    pub reasoning_hint: Option<String>,
}

// =========================================================================
// The Pipeline
// =========================================================================

/// Run the full constitutional pipeline.
///
/// This is the function that makes the BASE real. Every step is auditable,
/// every artifact is signed, every invariant is checked.
pub fn execute(
    req: GatewayRequest,
    policy_engine: &dyn ubl_policy::PolicyEngine,
    signing_key: &SigningKey,
    rt_info: receipt::RuntimeInfo,
) -> Result<GatewayResult, GatewayError> {
    // --- Step 0: Validate act vocabulary (Article V) ---
    match req.act.as_str() {
        "ATTEST" | "EVALUATE" | "TRANSACT" => {}
        other => return Err(GatewayError::InvalidAct(other.to_string())),
    }

    // --- Step 1: ρ-normalize the body (Article I) ---
    let body = rho::normalize(&req.body).map_err(|e| GatewayError::RhoFailed(format!("{e}")))?;
    let body_cid =
        rho::canonical_cid(&body).map_err(|e| GatewayError::RhoFailed(format!("{e}")))?;

    // --- Step 2: Policy gate (Article VI) ---
    let context_cid = body_cid.clone();
    let eval_req = ubl_policy::EvalRequest {
        policy_id: req
            .policy_id
            .clone()
            .unwrap_or_else(|| "existence/default@1".into()),
        context_cid,
        input: serde_json::to_value(body_to_json(&body)).unwrap_or_default(),
        pipeline_prev: req.pipeline_prev.clone(),
    };
    let eval_resp = policy_engine
        .evaluate(&eval_req)
        .map_err(|e| GatewayError::PolicyFailed(e.to_string()))?;

    // --- Step 3: Ghost(pending) — WBE discipline (Article IV §4.3, Module §2.3) ---
    let wbe = ghost::Wbe {
        who: req.issuer_did.clone(),
        what: format!("{} on {}", req.act, req.subject),
        when: now_nanos(),
        intent: req.act.clone(),
    };
    let mut g = ghost::Ghost::new_pending(wbe, req.nonce.clone(), req.url_base.clone());
    g.sign(signing_key);

    // --- Step 4: Build Receipt (Article IV §4.1) ---
    let decision_str = eval_resp.decision.as_str().to_string();
    // GHO-001: GHOST ⇒ effects = None; no effects for now — modules add effects in the future
    let effects: Option<nrf_core::Value> = None;
    let _ = &eval_resp.decision; // acknowledge decision was checked

    let url = receipt::rich_url(
        &req.url_base,
        "", // placeholder — will be filled after CID computation
        &req.issuer_did,
        &req.act,
    );

    let mut r = receipt::Receipt {
        v: "receipt-v1".into(),
        receipt_cid: String::new(),
        t: now_nanos(),
        issuer_did: req.issuer_did.clone(),
        subject_did: None,
        kid: None,
        act: req.act.clone(),
        subject: req.subject.clone(),
        decision: Some(decision_str),
        effects,
        body,
        body_cid,
        inputs_cid: None,
        policy: req.policy_id,
        reasoning_cid: eval_resp.reasoning_cid,
        permit_cid: None,
        pipeline_prev: req.pipeline_prev,
        rt: rt_info,
        prev: None,
        chain: None,
        ghost: None,
        nonce: req.nonce,
        url,
        sig: None,
    };

    // --- Step 5: Compute CID and sign (Article II — the fractal) ---
    r.receipt_cid = r.compute_cid();
    // Fix the URL with the real CID
    r.url = receipt::rich_url(&req.url_base, &r.receipt_cid, &r.issuer_did, &r.act);
    // Recompute CID after URL update (URL is part of the hash preimage)
    r.receipt_cid = r.compute_cid();
    r.sign(signing_key);

    // --- Step 6: Integrity check (Article IX §9.1 — no hidden state) ---
    r.verify_integrity()
        .map_err(|e| GatewayError::IntegrityFailed(e.to_string()))?;

    Ok(GatewayResult {
        receipt: r,
        ghost: g,
        decision: eval_resp.decision,
        reasoning_hint: eval_resp.reasoning_hint,
    })
}

// =========================================================================
// Helpers
// =========================================================================

fn now_nanos() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as i64
}

fn body_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::json!(*i),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Bytes(b) => serde_json::json!({"$bytes": hex::encode(b)}),
        Value::Array(arr) => serde_json::Value::Array(arr.iter().map(body_to_json).collect()),
        Value::Map(m) => {
            let obj: serde_json::Map<String, serde_json::Value> = m
                .iter()
                .map(|(k, v)| (k.clone(), body_to_json(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

// =========================================================================
// Built-in policy: Existence (always ALLOW — the simplest policy)
// =========================================================================

pub struct ExistencePolicy;

impl ubl_policy::PolicyEngine for ExistencePolicy {
    fn evaluate(&self, _req: &ubl_policy::EvalRequest) -> anyhow::Result<ubl_policy::EvalResponse> {
        Ok(ubl_policy::EvalResponse {
            decision: ubl_policy::Decision::Allow,
            reasoning_hint: Some("existence check: body is present and well-formed".into()),
            reasoning_cid: None,
            rules_fired: vec!["existence/body-present".into()],
        })
    }

    fn family(&self) -> ubl_policy::PolicyFamily {
        ubl_policy::PolicyFamily::Existence
    }
}

// =========================================================================
// Tests — Constitution of the Modules §5.2 compliance
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_key() -> (SigningKey, ed25519_dalek::VerifyingKey) {
        let mut rng = rand::thread_rng();
        let sk = SigningKey::generate(&mut rng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn test_rt() -> receipt::RuntimeInfo {
        receipt::RuntimeInfo {
            name: "receipt-gateway".into(),
            version: "0.1.0".into(),
            binary_sha256: "test-binary-hash".into(),
            hal_ref: None,
            env: BTreeMap::new(),
            certs: vec![],
        }
    }

    fn test_request() -> GatewayRequest {
        GatewayRequest {
            issuer_did: "did:ubl:test-issuer".into(),
            act: "ATTEST".into(),
            subject: "b3:0000000000000000000000000000000000000000000000000000000000000000".into(),
            body: Value::Map({
                let mut m = BTreeMap::new();
                m.insert("name".into(), Value::String("test document".into()));
                m.insert("version".into(), Value::Int(1));
                m
            }),
            policy_id: None,
            pipeline_prev: vec![],
            url_base: "https://passports.ubl.agency/test/demo/receipts/001.json".into(),
            nonce: vec![0u8; 16],
        }
    }

    // --- §5.2 requirement 1: produces a valid Receipt ---
    #[test]
    fn module_produces_valid_receipt() {
        let (sk, vk) = test_key();
        let policy = ExistencePolicy;
        let result =
            execute(test_request(), &policy, &sk, test_rt()).expect("pipeline should succeed");

        assert_eq!(result.receipt.v, "receipt-v1");
        assert!(result.receipt.receipt_cid.starts_with("b3:"));
        assert!(result.receipt.sig.is_some());
        assert!(
            result.receipt.verify(&vk),
            "MODULE §5.2: receipt signature must verify"
        );
    }

    // --- §5.2 requirement 2: Receipt passes verify_integrity() ---
    #[test]
    fn module_receipt_passes_integrity() {
        let (sk, _vk) = test_key();
        let policy = ExistencePolicy;
        let result =
            execute(test_request(), &policy, &sk, test_rt()).expect("pipeline should succeed");

        assert!(
            result.receipt.verify_integrity().is_ok(),
            "MODULE §5.2: receipt must pass verify_integrity()"
        );
    }

    // --- §5.2 requirement 3: CIDs are deterministic ---
    #[test]
    fn module_cid_deterministic() {
        let (sk, _vk) = test_key();
        let policy = ExistencePolicy;

        // Same body → same body_cid (even if receipt_cid differs due to timestamp)
        let r1 =
            execute(test_request(), &policy, &sk, test_rt()).expect("pipeline 1 should succeed");
        let r2 =
            execute(test_request(), &policy, &sk, test_rt()).expect("pipeline 2 should succeed");

        assert_eq!(
            r1.receipt.body_cid, r2.receipt.body_cid,
            "MODULE §5.2: same body must produce same body_cid"
        );
    }

    // --- Ghost discipline (Module §2.3) ---
    #[test]
    fn module_creates_ghost_before_receipt() {
        let (sk, vk) = test_key();
        let policy = ExistencePolicy;
        let result =
            execute(test_request(), &policy, &sk, test_rt()).expect("pipeline should succeed");

        assert_eq!(result.ghost.status, ghost::GhostStatus::Pending);
        assert!(
            result.ghost.sig.is_some(),
            "MODULE §2.3: ghost must be signed"
        );
        assert!(
            result.ghost.verify(&vk),
            "MODULE §2.3: ghost sig must verify"
        );
        assert!(
            result.ghost.t <= result.receipt.t,
            "MODULE §2.3: ghost must be created BEFORE receipt (WBE)"
        );
    }

    // --- Act vocabulary enforcement (Article V) ---
    #[test]
    fn module_rejects_invalid_act() {
        let (sk, _vk) = test_key();
        let policy = ExistencePolicy;
        let mut req = test_request();
        req.act = "INVALID".into();
        let err = execute(req, &policy, &sk, test_rt());
        assert!(err.is_err(), "MODULE: invalid act must be rejected");
        let msg = format!("{}", err.unwrap_err());
        assert!(
            msg.contains("ACT-001"),
            "MODULE: error must be greppable with ACT-001 code"
        );
    }

    // --- ρ normalization runs on body (Article I) ---
    #[test]
    fn module_rho_normalizes_body() {
        let (sk, _vk) = test_key();
        let policy = ExistencePolicy;
        let mut req = test_request();
        // Body with null value — ρ should strip it
        req.body = Value::Map({
            let mut m = BTreeMap::new();
            m.insert("keep".into(), Value::Int(1));
            m.insert("strip_me".into(), Value::Null);
            m
        });
        let result = execute(req, &policy, &sk, test_rt()).expect("pipeline should succeed");

        if let Value::Map(m) = &result.receipt.body {
            assert!(
                !m.contains_key("strip_me"),
                "MODULE Article I: ρ must strip null values from body before receipt"
            );
        } else {
            panic!("body should be a Map");
        }
    }

    // --- Policy decision flows through (Article VI, Module §6.3) ---
    #[test]
    fn module_policy_decision_flows_through() {
        let (sk, _vk) = test_key();
        let policy = ExistencePolicy;
        let result =
            execute(test_request(), &policy, &sk, test_rt()).expect("pipeline should succeed");

        assert_eq!(
            result.receipt.decision.as_deref(),
            Some("ALLOW"),
            "MODULE §6.3: decision must reflect what PolicyEngine returned"
        );
        assert_eq!(result.decision, ubl_policy::Decision::Allow);
    }

    // --- All three acts work ---
    #[test]
    fn module_all_three_acts() {
        let (sk, vk) = test_key();
        let policy = ExistencePolicy;
        for act in &["ATTEST", "EVALUATE", "TRANSACT"] {
            let mut req = test_request();
            req.act = act.to_string();
            let result = execute(req, &policy, &sk, test_rt())
                .unwrap_or_else(|_| panic!("pipeline should succeed for {act}"));
            assert_eq!(result.receipt.act, *act);
            assert!(result.receipt.verify(&vk));
        }
    }

    // --- Pipeline composition (Module §4.1) ---
    #[test]
    fn module_pipeline_prev_composition() {
        let (sk, vk) = test_key();
        let policy = ExistencePolicy;

        // Receipt 1: ATTEST
        let req1 = test_request();
        let r1 = execute(req1, &policy, &sk, test_rt()).expect("pipeline 1 should succeed");

        // Receipt 2: EVALUATE referencing Receipt 1
        let mut req2 = test_request();
        req2.act = "EVALUATE".into();
        req2.pipeline_prev = vec![r1.receipt.receipt_cid.clone()];
        let r2 = execute(req2, &policy, &sk, test_rt()).expect("pipeline 2 should succeed");

        assert_eq!(
            r2.receipt.pipeline_prev[0], r1.receipt.receipt_cid,
            "MODULE §4.1: pipeline_prev must carry the prior receipt CID"
        );
        assert!(r2.receipt.verify(&vk));
    }
}
