//! HTTP convenience endpoints wrapping the pure Capability modules.
//!
//! These endpoints provide a direct HTTP interface to each capability module,
//! complementing the pipeline-based `/modules/run` endpoint.
//!
//! Routes:
//!   POST /v1/intake          → cap-intake (normalize env via mapping DSL)
//!   POST /v1/permit/eval     → cap-permit (K-of-N consent evaluation)
//!   POST /v1/policy/apply    → cap-policy (rules DSL → ALLOW/DENY/REQUIRE)
//!   POST /v1/enrich/apply    → cap-enrich (render artifacts, webhook effects)
//!   POST /v1/transport/derive → cap-transport (build receipt, relay effects)
//!   POST /v1/llm/complete    → cap-llm (LLM assist, prompt-by-CID)
//!   POST /v1/llm/judge       → simple keyword judge (no LLM needed)

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use modules_core::{AssetResolver, Asset, CapInput, Capability, Cid, ExecutionMeta};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn now_nanos() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = (d.as_secs() as u128)
        .saturating_mul(1_000_000_000)
        .saturating_add(d.subsec_nanos() as u128);
    i64::try_from(nanos).unwrap_or(i64::MAX)
}

fn json_to_nrf(j: &Value) -> anyhow::Result<nrf1::Value> {
    use nrf1::Value as V;
    Ok(match j {
        Value::Null => V::Null,
        Value::Bool(b) => V::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                V::Int(i)
            } else {
                anyhow::bail!("numbers must fit i64 (no floats)");
            }
        }
        Value::String(s) => V::String(s.clone()),
        Value::Array(a) => V::Array(a.iter().map(json_to_nrf).collect::<anyhow::Result<Vec<_>>>()?),
        Value::Object(o) => {
            let mut m = BTreeMap::new();
            for (k, v) in o {
                m.insert(k.clone(), json_to_nrf(v)?);
            }
            V::Map(m)
        }
    })
}

fn make_meta(tenant: Option<&str>) -> ExecutionMeta {
    ExecutionMeta {
        run_id: uuid::Uuid::new_v4().to_string(),
        tenant: tenant.map(|s| s.to_string()),
        trace_id: None,
        ts_nanos: now_nanos(),
    }
}

#[derive(Clone)]
struct NullResolver;
impl AssetResolver for NullResolver {
    fn get(&self, _cid: &Cid) -> anyhow::Result<Asset> {
        anyhow::bail!("no assets configured")
    }
    fn box_clone(&self) -> Box<dyn AssetResolver> {
        Box::new(self.clone())
    }
}

fn cap_input(env_json: &Value, config: Value, tenant: Option<&str>) -> Result<CapInput, StatusCode> {
    let env = json_to_nrf(env_json).map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(CapInput {
        env,
        config,
        assets: Box::new(NullResolver),
        prev_receipts: vec![],
        meta: make_meta(tenant),
    })
}

fn cap_output_json(out: &modules_core::CapOutput) -> Value {
    let env_json = out
        .new_env
        .as_ref()
        .map(|v| ubl_json_view::to_json(v))
        .unwrap_or(Value::Null);

    let verdict = out
        .verdict
        .as_ref()
        .map(|v| format!("{:?}", v).to_uppercase())
        .unwrap_or_else(|| "NONE".into());

    let metrics: Value = out
        .metrics
        .iter()
        .map(|(k, v)| (k.clone(), json!(*v)))
        .collect::<serde_json::Map<String, Value>>()
        .into();

    let artifacts: Vec<Value> = out
        .artifacts
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "mime": a.mime,
                "size": a.bytes.len(),
            })
        })
        .collect();

    let effects: Vec<Value> = out
        .effects
        .iter()
        .map(|e| json!(format!("{:?}", e).chars().take(120).collect::<String>()))
        .collect();

    json!({
        "env": env_json,
        "verdict": verdict,
        "metrics": metrics,
        "artifacts": artifacts,
        "effects_count": effects.len(),
    })
}

// ---------------------------------------------------------------------------
// POST /v1/intake
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct IntakeReq {
    env: Value,
    #[serde(default)]
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn intake_handler(Json(req): Json<IntakeReq>) -> impl IntoResponse {
    let config = if req.config.is_null() {
        json!({"mapping": [], "defaults": {}})
    } else {
        req.config
    };

    let input = match cap_input(&req.env, config, req.tenant.as_deref()) {
        Ok(i) => i,
        Err(s) => return (s, Json(json!({"error": "invalid env"}))).into_response(),
    };

    let module = cap_intake::IntakeModule;
    match module.execute(input) {
        Ok(out) => {
            let mut resp = cap_output_json(&out);
            // For intake, also return the NRF-canonical JSON view of the new env
            if let Some(new_env) = &out.new_env {
                let canon_bytes = nrf1::encode(new_env);
                let id_hex = hex::encode(blake3::hash(&canon_bytes).as_bytes());
                resp["id_b3"] = json!(format!("b3:{}", id_hex));
            }
            (StatusCode::OK, Json(resp)).into_response()
        }
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/permit/eval
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PermitEvalReq {
    #[serde(default)]
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn permit_eval_handler(Json(req): Json<PermitEvalReq>) -> impl IntoResponse {
    let config = if req.config.is_null() {
        json!({
            "quorum": {"k": 1, "n": 1, "roles": ["default"]},
            "ttl_sec": 3600,
            "timeout_action": "DENY"
        })
    } else {
        req.config
    };

    let input = match cap_input(&json!({}), config, req.tenant.as_deref()) {
        Ok(i) => i,
        Err(s) => return (s, Json(json!({"error": "invalid input"}))).into_response(),
    };

    let module = cap_permit::PermitModule;
    match module.execute(input) {
        Ok(out) => (StatusCode::OK, Json(cap_output_json(&out))).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/policy/apply
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PolicyApplyReq {
    env: Value,
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn policy_apply_handler(Json(req): Json<PolicyApplyReq>) -> impl IntoResponse {
    let input = match cap_input(&req.env, req.config, req.tenant.as_deref()) {
        Ok(i) => i,
        Err(s) => return (s, Json(json!({"error": "invalid env"}))).into_response(),
    };

    let module = cap_policy::PolicyModule;
    match module.execute(input) {
        Ok(out) => (StatusCode::OK, Json(cap_output_json(&out))).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/enrich/apply
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct EnrichApplyReq {
    env: Value,
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn enrich_apply_handler(Json(req): Json<EnrichApplyReq>) -> impl IntoResponse {
    let input = match cap_input(&req.env, req.config, req.tenant.as_deref()) {
        Ok(i) => i,
        Err(s) => return (s, Json(json!({"error": "invalid env"}))).into_response(),
    };

    let module = cap_enrich::EnrichModule;
    match module.execute(input) {
        Ok(out) => (StatusCode::OK, Json(cap_output_json(&out))).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/transport/derive
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TransportDeriveReq {
    env: Value,
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn transport_derive_handler(Json(req): Json<TransportDeriveReq>) -> impl IntoResponse {
    let input = match cap_input(&req.env, req.config, req.tenant.as_deref()) {
        Ok(i) => i,
        Err(s) => return (s, Json(json!({"error": "invalid env"}))).into_response(),
    };

    let module = cap_transport::TransportModule;
    match module.execute(input) {
        Ok(out) => (StatusCode::OK, Json(cap_output_json(&out))).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/llm/complete
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LlmCompleteReq {
    env: Value,
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn llm_complete_handler(Json(req): Json<LlmCompleteReq>) -> impl IntoResponse {
    let input = match cap_input(&req.env, req.config, req.tenant.as_deref()) {
        Ok(i) => i,
        Err(s) => return (s, Json(json!({"error": "invalid env"}))).into_response(),
    };

    let module = cap_llm::LlmModule;
    match module.execute(input) {
        Ok(out) => (StatusCode::OK, Json(cap_output_json(&out))).into_response(),
        Err(e) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/llm/judge — simple keyword judge (no LLM needed)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct JudgeReq {
    expected_keywords: Vec<String>,
    text: String,
}

#[derive(Serialize)]
struct JudgeResp {
    ok: bool,
    missing: Vec<String>,
}

async fn judge_handler(Json(req): Json<JudgeReq>) -> Json<JudgeResp> {
    let mut missing = vec![];
    for k in &req.expected_keywords {
        if !req.text.to_lowercase().contains(&k.to_lowercase()) {
            missing.push(k.clone());
        }
    }
    Json(JudgeResp {
        ok: missing.is_empty(),
        missing,
    })
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn cap_http_router() -> Router {
    Router::new()
        .route("/v1/intake", post(intake_handler))
        .route("/v1/permit/eval", post(permit_eval_handler))
        .route("/v1/policy/apply", post(policy_apply_handler))
        .route("/v1/enrich/apply", post(enrich_apply_handler))
        .route("/v1/transport/derive", post(transport_derive_handler))
        .route("/v1/llm/complete", post(llm_complete_handler))
        .route("/v1/llm/judge", post(judge_handler))
}
