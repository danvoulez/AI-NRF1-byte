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
// POST /v1/llm/complete  (direct provider call OR Capability-based)
//
// If `prompt` is present → direct provider call (OpenAI/Ollama).
// If `env` + `config` are present → Capability-based (cap-llm module).
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct LlmCompleteReq {
    // --- Direct provider mode ---
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    json_mode: bool,
    // --- Capability mode (legacy) ---
    #[serde(default)]
    env: Value,
    #[serde(default)]
    config: Value,
    #[serde(default)]
    tenant: Option<String>,
}

async fn llm_complete_handler(Json(req): Json<LlmCompleteReq>) -> impl IntoResponse {
    // Direct provider mode: if `prompt` is present, call the LLM provider directly.
    if let Some(prompt) = &req.prompt {
        return llm_direct_call(prompt, &req).await;
    }

    // Capability mode: delegate to cap-llm module.
    let input = match cap_input(&req.env, req.config.clone(), req.tenant.as_deref()) {
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

async fn llm_direct_call(prompt: &str, req: &LlmCompleteReq) -> axum::response::Response {
    use module_runner::adapters::llm::*;

    // Load provider config
    let cfg_path = std::env::var("LLM_PROVIDERS_PATH")
        .unwrap_or_else(|_| "configs/llm/providers.yaml".into());
    let cfg: ProvidersCfg = match std::fs::read_to_string(&cfg_path)
        .map_err(anyhow::Error::from)
        .and_then(|s| serde_yaml::from_str(&s).map_err(anyhow::Error::from))
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("llm config: {e}")})),
            )
                .into_response();
        }
    };

    let model = req.model.as_deref().unwrap_or(&cfg.defaults.model);
    let max_tokens = req.max_tokens.unwrap_or(cfg.defaults.max_tokens.unwrap_or(800));
    let temperature = req.temperature.or(cfg.defaults.temperature);

    // Determine which provider to use based on model name
    let is_ollama_model = cfg
        .providers
        .ollama
        .as_ref()
        .filter(|o| o.enabled)
        .map(|o| o.models.iter().any(|m| m == model))
        .unwrap_or(false);

    let result = if is_ollama_model {
        let oc = match cfg.providers.ollama.as_ref().filter(|c| c.enabled) {
            Some(c) => c,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "ollama provider not enabled"})),
                )
                    .into_response();
            }
        };
        #[cfg(feature = "modules")]
        {
            let provider = ollama::OllamaProvider::new(
                oc.base_url.clone(),
                oc.pricing_per_1k.input_usd,
                oc.pricing_per_1k.output_usd,
            );
            provider
                .invoke_ext(model, prompt, max_tokens, temperature, req.json_mode)
                .await
        }
        #[cfg(not(feature = "modules"))]
        {
            let _ = oc;
            Err(anyhow::anyhow!("live providers require modules feature"))
        }
    } else {
        let oc = match cfg.providers.openai.as_ref().filter(|c| c.enabled) {
            Some(c) => c,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "openai provider not enabled"})),
                )
                    .into_response();
            }
        };
        #[cfg(feature = "modules")]
        {
            let api_key = match std::env::var(&oc.api_key_env) {
                Ok(k) => k,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("env {} not set", oc.api_key_env)})),
                    )
                        .into_response();
                }
            };
            let provider = openai::OpenAiProvider::new(
                oc.base_url.clone(),
                api_key,
                oc.pricing_per_1k.input_usd,
                oc.pricing_per_1k.output_usd,
            );
            provider
                .invoke_ext(model, prompt, max_tokens, temperature, req.json_mode)
                .await
        }
        #[cfg(not(feature = "modules"))]
        {
            let _ = oc;
            Err(anyhow::anyhow!("live providers require modules feature"))
        }
    };

    match result {
        Ok(out) => (
            StatusCode::OK,
            Json(json!({
                "text": out.text,
                "model": model,
                "tokens_in": out.tokens_in,
                "tokens_out": out.tokens_out,
                "tokens_used": out.tokens_used,
                "cost_usd": out.cost_usd,
                "finish_reason": out.finish_reason,
                "cached": out.cached,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
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
// POST /v1/pricing/price
// ---------------------------------------------------------------------------

async fn pricing_price_handler(
    Json(req): Json<cap_pricing::api::PriceReq>,
) -> impl IntoResponse {
    match cap_pricing::price_one(&req) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/pricing/quote (scenario pricing)
// ---------------------------------------------------------------------------

async fn pricing_quote_handler(
    Json(req): Json<cap_pricing::api::ScenarioReq>,
) -> impl IntoResponse {
    match cap_pricing::price_scenario(&req) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/quote/create
// ---------------------------------------------------------------------------

async fn quote_create_handler(
    Json(req): Json<cap_quote::api::QuoteCreateReq>,
) -> impl IntoResponse {
    match cap_quote::create_quote(&req) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// GET /v1/quote/get/:id
// ---------------------------------------------------------------------------

async fn quote_get_handler(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> impl IntoResponse {
    match cap_quote::get_quote_by_id(id) {
        Some(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "quote not found"})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/quote/reprice
// ---------------------------------------------------------------------------

async fn quote_reprice_handler(
    Json(req): Json<cap_quote::api::QuoteRepriceReq>,
) -> impl IntoResponse {
    match cap_quote::reprice_quote(req.id) {
        Some(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "quote not found"})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/invoice/create_from_quote
// ---------------------------------------------------------------------------

async fn invoice_create_handler(
    Json(req): Json<cap_invoice::CreateFromQuoteReq>,
) -> impl IntoResponse {
    match cap_invoice::create_from_quote(&req) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        Err(e) => {
            let code = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };
            (code, Json(json!({"error": format!("{e}")}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// GET /v1/invoice/get/:id
// ---------------------------------------------------------------------------

async fn invoice_get_handler(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> impl IntoResponse {
    match cap_invoice::get_invoice(id) {
        Ok(Some(resp)) => {
            (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "invoice not found"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// POST /v1/invoice/approve
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct InvoiceActionReq {
    id: uuid::Uuid,
}

async fn invoice_approve_handler(Json(req): Json<InvoiceActionReq>) -> impl IntoResponse {
    match cap_invoice::approve_invoice(req.id) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        Err(e) => {
            let code = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (code, Json(json!({"error": format!("{e}")}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// POST /v1/invoice/reject
// ---------------------------------------------------------------------------

async fn invoice_reject_handler(Json(req): Json<InvoiceActionReq>) -> impl IntoResponse {
    match cap_invoice::reject_invoice(req.id) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::to_value(&resp).unwrap())).into_response(),
        Err(e) => {
            let code = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (code, Json(json!({"error": format!("{e}")}))).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn cap_http_router() -> Router {
    // Load pricing config if PRICING_PATH is set
    if let Ok(p) = std::env::var("PRICING_PATH") {
        if let Err(e) = cap_pricing::load_pricing_from(&p) {
            tracing::warn!("failed to load pricing config from {p}: {e}");
        }
    }
    // Initialize invoice store
    cap_invoice::init_store();

    Router::new()
        .route("/v1/intake", post(intake_handler))
        .route("/v1/permit/eval", post(permit_eval_handler))
        .route("/v1/policy/apply", post(policy_apply_handler))
        .route("/v1/enrich/apply", post(enrich_apply_handler))
        .route("/v1/transport/derive", post(transport_derive_handler))
        .route("/v1/llm/complete", post(llm_complete_handler))
        .route("/v1/llm/judge", post(judge_handler))
        // Pricing
        .route("/v1/pricing/price", post(pricing_price_handler))
        .route("/v1/pricing/quote", post(pricing_quote_handler))
        // Quote
        .route("/v1/quote/create", post(quote_create_handler))
        .route("/v1/quote/get/:id", axum::routing::get(quote_get_handler))
        .route("/v1/quote/reprice", post(quote_reprice_handler))
        // Invoice
        .route("/v1/invoice/create_from_quote", post(invoice_create_handler))
        .route("/v1/invoice/get/:id", axum::routing::get(invoice_get_handler))
        .route("/v1/invoice/approve", post(invoice_approve_handler))
        .route("/v1/invoice/reject", post(invoice_reject_handler))
}
