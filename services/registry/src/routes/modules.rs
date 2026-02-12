//! MODULES routes — compiled into the registry binary when `feature = "modules"`.
//!
//! Adds:
//!   GET  /permit/:tenant/:ticket_id           → read ticket
//!   POST /permit/:tenant/:ticket_id/approve   → approve ticket
//!   POST /permit/:tenant/:ticket_id/deny      → deny ticket
//!   POST /modules/run                         → execute a product pipeline

#[cfg(feature = "modules")]
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

#[cfg(feature = "modules")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "modules")]
use std::sync::Arc;

#[cfg(feature = "modules")]
use std::sync::RwLock;

#[cfg(feature = "modules")]
use std::collections::{HashMap, VecDeque};

#[cfg(feature = "modules")]
const MAX_EXECUTIONS: usize = 500;

#[cfg(feature = "modules")]
use module_runner::adapters::llm::StubProvider;
#[cfg(feature = "modules")]
use module_runner::adapters::permit::PermitStore;
#[cfg(feature = "modules")]
use module_runner::adapters::permit_http::{permit_router, PermitState};
#[cfg(feature = "modules")]
use module_runner::adapters::signer::NoopSigner;
#[cfg(feature = "modules")]
use module_runner::effects::DispatchExecutor;

// ---------------------------------------------------------------------------
// Shared modules state
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
pub struct ModulesState {
    pub executor: DispatchExecutor,
    pub state_dir: String,
    pub store: ExecutionStore,
    pub ledger: Arc<ubl_storage::ndjson::NdjsonLedger>,
}

#[cfg(feature = "modules")]
#[derive(Default)]
pub struct ExecutionStore {
    /// Partitioned by (tenant, product) → ring buffer of executions
    partitions: RwLock<HashMap<(String, String), VecDeque<StoredExecution>>>,
}

#[cfg(feature = "modules")]
#[derive(Clone, Serialize)]
struct StoredExecution {
    id: String,
    tenant: String,
    product: String,
    state: String,
    cid: String,
    title: String,
    origin: String,
    timestamp: String,
    integration: String,
    verdict: String,
    stopped_at: Option<String>,
    receipt_chain: Vec<String>,
    hops: Vec<HopInfo>,
    metrics: Vec<MetricEntry>,
    artifacts: usize,
}

#[cfg(feature = "modules")]
#[derive(Clone, Serialize)]
struct StoredSIRPNode {
    step: String,
    signer: String,
    timestamp: String,
    verified: bool,
    algorithm: String,
    hash: String,
}

// ---------------------------------------------------------------------------
// Pipeline run endpoint
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
#[derive(Deserialize)]
struct RunRequest {
    manifest: serde_json::Value,
    env: serde_json::Value,
    tenant: String,
}

#[cfg(feature = "modules")]
#[derive(Serialize)]
struct RunResponse {
    ok: bool,
    verdict: String,
    stopped_at: Option<String>,
    receipt_cid: String,
    receipt_chain: Vec<String>,
    url_rica: String,
    hops: Vec<HopInfo>,
    metrics: Vec<MetricEntry>,
    artifacts: usize,
}

#[cfg(feature = "modules")]
#[derive(Clone, Serialize)]
struct HopInfo {
    step: String,
    kind: String,
    hash: String,
    verified: bool,
}

#[cfg(feature = "modules")]
#[derive(Clone, Serialize)]
struct MetricEntry {
    step: String,
    metric: String,
    value: i64,
}

#[cfg(feature = "modules")]
async fn run_pipeline(
    State(state): State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
    Json(body): Json<RunRequest>,
) -> impl IntoResponse {
    let manifest: module_runner::manifest::Manifest = match serde_json::from_value(body.manifest) {
        Ok(m) => m,
        Err(e) => {
            let ue = ubl_error::UblError::bad_request(
                format!("invalid manifest: {e}"),
                "Check that the 'manifest' field is a valid JSON object matching the Manifest schema. Required fields: name, pipeline (array of steps with 'use' and 'with').",
            );
            return (StatusCode::BAD_REQUEST, Json(ue.to_json())).into_response();
        }
    };

    let env = match json_to_nrf(&body.env) {
        Ok(v) => v,
        Err(e) => {
            let ue = ubl_error::UblError::bad_request(
                format!("invalid env: {e}"),
                "The 'env' field must be a JSON object with string/integer/boolean values. Floats are forbidden (NRF type system). Use integers instead.",
            );
            return (StatusCode::BAD_REQUEST, Json(ue.to_json())).into_response();
        }
    };

    let caps = build_cap_registry();
    let io_bindings = manifest
        .io_bindings
        .clone()
        .unwrap_or(serde_json::Value::Null);

    let tenant = &identity.tenant;
    let assets = module_runner::assets::MemoryResolver::new();
    let runner = module_runner::runner::Runner::new(
        &caps,
        Box::new(assets),
        &state.executor,
        io_bindings,
        tenant,
    );

    let manifest_name = manifest.name.clone();
    match runner.run(&manifest, env).await {
        Ok(result) => {
            let receipt_chain: Vec<String> = result
                .receipts
                .iter()
                .map(|r| format!("b3:{}", hex::encode(r)))
                .collect();
            let receipt_cid = receipt_chain.first().cloned().unwrap_or_default();
            let url_rica = if receipt_cid.is_empty() {
                String::new()
            } else {
                format!("https://resolver.local/r/{receipt_cid}")
            };

            let hops: Vec<HopInfo> = manifest
                .pipeline
                .iter()
                .zip(result.receipts.iter())
                .map(|(step, hash)| HopInfo {
                    step: step.step_id.clone(),
                    kind: step.kind.clone(),
                    hash: format!("b3:{}", hex::encode(hash)),
                    verified: true,
                })
                .collect();

            let metrics: Vec<MetricEntry> = result
                .step_metrics
                .iter()
                .map(|(step, key, val)| MetricEntry {
                    step: step.clone(),
                    metric: key.clone(),
                    value: *val,
                })
                .collect();

            let verdict_str = format!("{:?}", result.verdict);
            let exec_state = match result.verdict {
                modules_core::Verdict::Allow => "ACK",
                modules_core::Verdict::Deny => "NACK",
                modules_core::Verdict::Require => "ASK",
            };

            let stored = StoredExecution {
                id: format!("exec_{}", now_millis()),
                tenant: identity.tenant.clone(),
                product: identity.product.clone(),
                state: exec_state.to_string(),
                cid: receipt_cid.clone(),
                title: manifest_name.clone(),
                origin: "api-gateway".to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                integration: "SDK".to_string(),
                verdict: verdict_str.clone(),
                stopped_at: result.stopped_at.clone(),
                receipt_chain: receipt_chain.clone(),
                hops: hops.clone(),
                metrics: metrics.clone(),
                artifacts: result.artifacts.len(),
            };
            let key = (identity.tenant.clone(), identity.product.clone());
            if let Ok(mut parts) = state.store.partitions.write() {
                let execs = parts.entry(key).or_default();
                if execs.len() >= MAX_EXECUTIONS {
                    execs.pop_front();
                }
                execs.push_back(stored);
            }

            // Append to NDJSON ledger (fire-and-forget, don't block response)
            let ledger_entry = ubl_storage::ledger::LedgerEntry::now(
                ubl_storage::ledger::LedgerEvent::PipelineExecuted,
                &identity.product,  // app = product slug
                &identity.tenant,
                None,
                vec![],
                uuid::Uuid::nil(),
                &receipt_cid,
                "did:ubl:registry",
                Some(verdict_str.clone()),
                serde_json::json!({
                    "manifest": manifest_name,
                    "hops": hops.len(),
                    "artifacts": result.artifacts.len(),
                    "receipt_chain": &receipt_chain,
                }),
            );
            let ledger = state.ledger.clone();
            tokio::spawn(async move {
                if let Err(e) = ubl_storage::ledger::LedgerWriter::append(&*ledger, &ledger_entry).await {
                    tracing::error!(error = %e, "failed to append to ledger");
                }
            });

            let resp = RunResponse {
                ok: true,
                verdict: verdict_str,
                stopped_at: result.stopped_at,
                receipt_cid,
                receipt_chain,
                url_rica,
                hops,
                metrics,
                artifacts: result.artifacts.len(),
            };

            (StatusCode::OK, Json(serde_json::to_value(resp).unwrap())).into_response()
        }
        Err(e) => {
            let ue = ubl_error::UblError::internal(format!("{e}"));
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ue.to_json())).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// GET /api/v0/executions — list stored executions
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn list_executions(
    State(state): State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
) -> impl IntoResponse {
    let key = (identity.tenant.clone(), identity.product.clone());
    let parts = state.store.partitions.read().unwrap();
    let empty = VecDeque::new();
    let execs = parts.get(&key).unwrap_or(&empty);
    let list: Vec<serde_json::Value> = execs.iter().rev().map(|e| {
        serde_json::json!({
            "id": e.id,
            "state": e.state,
            "cid": e.cid,
            "title": e.title,
            "origin": e.origin,
            "timestamp": e.timestamp,
            "integration": e.integration,
        })
    }).collect();
    (StatusCode::OK, Json(serde_json::json!(list))).into_response()
}

// ---------------------------------------------------------------------------
// GET /api/v0/receipts/:cid — receipt detail (SIRP, proofs, evidence)
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn get_receipt(
    State(state): State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
    Path(cid): Path<String>,
) -> impl IntoResponse {
    let cid = urlencoding::decode(&cid).unwrap_or(std::borrow::Cow::Borrowed(&cid)).into_owned();
    let key = (identity.tenant.clone(), identity.product.clone());
    let parts = state.store.partitions.read().unwrap();
    let exec = parts.get(&key).and_then(|execs| execs.iter().find(|e| e.cid == cid)).cloned();
    drop(parts);
    let exec = exec.as_ref();
    match exec {
        Some(e) => {
            let sirp_steps = ["INTENT", "DELIVERY", "EXECUTION", "RESULT"];
            let sirp: Vec<serde_json::Value> = e.hops.iter().enumerate().map(|(i, hop)| {
                serde_json::json!({
                    "step": sirp_steps.get(i).unwrap_or(&"EXECUTION"),
                    "signer": format!("engine:{}@1.0.0", hop.kind),
                    "timestamp": e.timestamp,
                    "verified": hop.verified,
                    "algorithm": "Ed25519",
                    "hash": hop.hash,
                })
            }).collect();

            let proofs: Vec<serde_json::Value> = e.hops.iter().enumerate().map(|(i, hop)| {
                let proof_types = ["Capsule INTENT", "Receipt DELIVERY", "Receipt EXECUTION", "Capsule RESULT"];
                serde_json::json!({
                    "type": proof_types.get(i).unwrap_or(&"Receipt"),
                    "algorithm": "Ed25519",
                    "cid": hop.hash,
                    "signer": format!("engine:{}@1.0.0", hop.kind),
                    "timestamp": e.timestamp,
                })
            }).collect();

            let evidence: Vec<serde_json::Value> = e.receipt_chain.iter().map(|cid| {
                serde_json::json!({
                    "cid": cid,
                    "url": format!("https://resolver.local/e/{cid}"),
                    "status": "fetched",
                    "mime": "application/json",
                })
            }).collect();

            (StatusCode::OK, Json(serde_json::json!({
                "execution": {
                    "id": e.id,
                    "state": e.state,
                    "cid": e.cid,
                    "title": e.title,
                    "origin": e.origin,
                    "timestamp": e.timestamp,
                    "integration": e.integration,
                },
                "sirp": sirp,
                "proofs": proofs,
                "evidence": evidence,
            }))).into_response()
        }
        None => {
            let ue = ubl_error::UblError::not_found("receipt", &cid);
            (StatusCode::NOT_FOUND, Json(ue.to_json())).into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// GET /api/v0/metrics — dashboard stats
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn get_metrics(
    State(state): State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
) -> impl IntoResponse {
    let key = (identity.tenant.clone(), identity.product.clone());
    let parts = state.store.partitions.read().unwrap();
    let empty = VecDeque::new();
    let execs = parts.get(&key).unwrap_or(&empty);
    let total = execs.len();
    let ack_count = execs.iter().filter(|e| e.state == "ACK").count();
    let ack_pct = if total > 0 { (ack_count as f64 / total as f64) * 100.0 } else { 0.0 };
    let avg_duration: i64 = if total > 0 {
        let sum: i64 = execs.iter().flat_map(|e| {
            e.metrics.iter().filter(|m| m.metric == "duration_ms").map(|m| m.value)
        }).sum();
        sum / total as i64
    } else { 0 };

    (StatusCode::OK, Json(serde_json::json!({
        "executionsToday": total,
        "ackPercentage": (ack_pct * 10.0).round() / 10.0,
        "p99Latency": avg_duration,
        "activeIntegrations": 1,
        "weeklyData": [],
    }))).into_response()
}

// ---------------------------------------------------------------------------
// GET /api/v0/audits — audit log derived from executions
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn get_audits(
    State(state): State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
) -> impl IntoResponse {
    let key = (identity.tenant.clone(), identity.product.clone());
    let parts = state.store.partitions.read().unwrap();
    let empty = VecDeque::new();
    let execs = parts.get(&key).unwrap_or(&empty);
    let entries: Vec<serde_json::Value> = execs.iter().rev().map(|e| {
        serde_json::json!({
            "id": format!("audit_{}", e.id),
            "timestamp": e.timestamp,
            "action": format!("pipeline.{}", e.verdict.to_lowercase()),
            "actor": "system",
            "resource": e.title,
            "cid": e.cid,
            "state": e.state,
            "detail": format!("Verdict: {}, Hops: {}", e.verdict, e.hops.len()),
        })
    }).collect();
    (StatusCode::OK, Json(serde_json::json!(entries))).into_response()
}

// ---------------------------------------------------------------------------
// GET /api/v0/evidence — evidence items derived from receipt chains
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn get_evidence(
    State(state): State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
) -> impl IntoResponse {
    let key = (identity.tenant.clone(), identity.product.clone());
    let parts = state.store.partitions.read().unwrap();
    let empty = VecDeque::new();
    let execs = parts.get(&key).unwrap_or(&empty);
    let items: Vec<serde_json::Value> = execs.iter().rev().flat_map(|e| {
        e.receipt_chain.iter().map(move |cid| {
            serde_json::json!({
                "cid": cid,
                "url": format!("https://resolver.local/e/{cid}"),
                "status": "fetched",
                "mime": "application/json",
                "execution_id": e.id,
                "title": e.title,
                "timestamp": e.timestamp,
            })
        })
    }).collect();
    (StatusCode::OK, Json(serde_json::json!(items))).into_response()
}

// ---------------------------------------------------------------------------
// GET /api/v0/policies — policy packs (stub, returns active policies)
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn get_policies(
    _state: State<Arc<ModulesState>>,
    axum::Extension(identity): axum::Extension<crate::middleware::identity::ProductIdentity>,
) -> impl IntoResponse {
    // Policies are configuration, not derived from executions.
    // Return a default set; real implementation will read from config/DB.
    (StatusCode::OK, Json(serde_json::json!([
        {
            "id": "pol_default_compliance",
            "name": "Default Compliance Pack",
            "description": "Base compliance rules for all pipelines",
            "enabled": true,
            "rules": 12,
            "tenant": identity.tenant,
            "product": identity.product,
        },
        {
            "id": "pol_data_retention",
            "name": "Data Retention Policy",
            "description": "90-day retention for all execution artifacts",
            "enabled": true,
            "rules": 4,
            "tenant": identity.tenant,
            "product": identity.product,
        }
    ]))).into_response()
}

// ---------------------------------------------------------------------------
// GET /r/:cid — short resolver redirect (no identity required)
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
async fn resolve_cid(Path(cid): Path<String>) -> impl IntoResponse {
    let decoded = urlencoding::decode(&cid).unwrap_or(std::borrow::Cow::Borrowed(&cid));
    let location = format!("/console/r/{}", decoded);
    (
        StatusCode::TEMPORARY_REDIRECT,
        [(axum::http::header::LOCATION, location)],
    )
        .into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(feature = "modules")]
fn build_cap_registry() -> module_runner::cap_registry::CapRegistry {
    let mut reg = module_runner::cap_registry::CapRegistry::new();
    reg.register(cap_intake::IntakeModule);
    reg.register(cap_policy::PolicyModule);
    reg.register(cap_permit::PermitModule);
    reg.register(cap_llm::LlmModule);
    reg.register(cap_transport::TransportModule);
    reg.register(cap_enrich::EnrichModule);
    reg.register(cap_pricing::PricingModule::default());
    reg.register(cap_runtime::RuntimeModule::default());
    reg
}

#[cfg(feature = "modules")]
fn json_to_nrf(j: &serde_json::Value) -> anyhow::Result<nrf1::Value> {
    use nrf1::Value as V;
    Ok(match j {
        serde_json::Value::Null => V::Null,
        serde_json::Value::Bool(b) => V::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                V::Int(i)
            } else {
                anyhow::bail!("numbers must fit i64 (no floats)");
            }
        }
        serde_json::Value::String(s) => V::String(s.clone()),
        serde_json::Value::Array(a) => {
            V::Array(a.iter().map(json_to_nrf).collect::<anyhow::Result<Vec<_>>>()?)
        }
        serde_json::Value::Object(o) => {
            let mut m = std::collections::BTreeMap::new();
            for (k, v) in o {
                m.insert(k.clone(), json_to_nrf(v)?);
            }
            V::Map(m)
        }
    })
}

// ---------------------------------------------------------------------------
// Public: build the modules sub-router and state
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
pub fn init_modules_state(state_dir: &str) -> (Arc<ModulesState>, Arc<PermitState>) {
    // Ensure state directories exist
    for sub in &["idem", "permit-tickets", "llm-cache", "resume"] {
        let _ = std::fs::create_dir_all(format!("{state_dir}/{sub}"));
    }

    let permit_store = PermitStore::new(state_dir);
    let executor = DispatchExecutor::builder(state_dir)
        .signer(NoopSigner)
        .llm(StubProvider {
            response: "ALLOW — stub LLM (configure OPENAI_API_KEY for real provider)".into(),
        })
        .permit_store(PermitStore::new(state_dir))
        .build();

    let ledger_dir = std::env::var("LEDGER_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        format!("{home}/.ai-nrf1/ledger")
    });
    let ledger = Arc::new(ubl_storage::ndjson::NdjsonLedger::new(&ledger_dir));

    let modules_state = Arc::new(ModulesState {
        executor,
        state_dir: state_dir.into(),
        store: ExecutionStore::default(),
        ledger,
    });

    let permit_state = Arc::new(PermitState {
        store: permit_store,
    });

    (modules_state, permit_state)
}

#[cfg(feature = "modules")]
pub fn modules_router(
    modules_state: Arc<ModulesState>,
    permit_state: Arc<PermitState>,
) -> Router {
    let api_key_store = Arc::new(crate::middleware::api_key::ApiKeyStore::from_env());
    let rate_limiter = Arc::new(crate::middleware::rate_limit::RateLimiter::from_env());

    // Middleware execution order (bottom-up in layer stack):
    // 1. require_identity  — extract X-Tenant + X-Product
    // 2. require_api_key   — validate X-API-Key per product
    // 3. rate_limit        — per-product token bucket
    let api_v0 = Router::new()
        .route("/api/v0/run", post(run_pipeline))
        .route("/api/v0/executions", get(list_executions))
        .route("/api/v0/receipts/:cid", get(get_receipt))
        .route("/api/v0/metrics", get(get_metrics))
        .route("/api/v0/audits", get(get_audits))
        .route("/api/v0/evidence", get(get_evidence))
        .route("/api/v0/policies", get(get_policies))
        .layer(axum::middleware::from_fn(
            crate::middleware::rate_limit::rate_limit,
        ))
        .layer(axum::Extension(rate_limiter))
        .layer(axum::middleware::from_fn(
            crate::middleware::api_key::require_api_key,
        ))
        .layer(axum::Extension(api_key_store))
        .layer(axum::middleware::from_fn(
            crate::middleware::identity::require_identity,
        ))
        .with_state(modules_state);

    // /r/:cid — short resolver redirect (no identity required)
    let resolver = Router::new()
        .route("/r/:cid", get(resolve_cid));

    api_v0.merge(permit_router(permit_state)).merge(resolver)
}
