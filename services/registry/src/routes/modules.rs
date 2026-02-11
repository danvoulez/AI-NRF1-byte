//! MODULES routes — compiled into the registry binary when `feature = "modules"`.
//!
//! Adds:
//!   GET  /permit/:tenant/:ticket_id           → read ticket
//!   POST /permit/:tenant/:ticket_id/approve   → approve ticket
//!   POST /permit/:tenant/:ticket_id/deny      → deny ticket
//!   POST /modules/run                         → execute a product pipeline

#[cfg(feature = "modules")]
use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};

#[cfg(feature = "modules")]
use serde::Deserialize;

#[cfg(feature = "modules")]
use std::sync::Arc;

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
}

// ---------------------------------------------------------------------------
// Pipeline run endpoint
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
#[derive(Deserialize)]
struct RunRequest {
    manifest_path: String,
    env: serde_json::Value,
    tenant: String,
}

#[cfg(feature = "modules")]
async fn run_pipeline(
    State(state): State<Arc<ModulesState>>,
    Json(body): Json<RunRequest>,
) -> impl IntoResponse {
    let manifest_raw = match std::fs::read_to_string(&body.manifest_path) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("cannot read manifest: {e}")})),
            )
                .into_response();
        }
    };
    let manifest: module_runner::manifest::Manifest = match serde_json::from_str(&manifest_raw) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid manifest: {e}")})),
            )
                .into_response();
        }
    };

    let env = match json_to_nrf(&body.env) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid env: {e}")})),
            )
                .into_response();
        }
    };

    let caps = build_cap_registry();
    let io_bindings = manifest
        .io_bindings
        .clone()
        .unwrap_or(serde_json::Value::Null);

    let assets = module_runner::assets::MemoryResolver::new();
    let runner = module_runner::runner::Runner::new(
        &caps,
        Box::new(assets),
        &state.executor,
        io_bindings,
        &body.tenant,
    );

    match runner.run(&manifest, env).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "verdict": format!("{:?}", result.verdict),
                "stopped_at": result.stopped_at,
                "receipts": result.receipts.len(),
                "artifacts": result.artifacts.len(),
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(feature = "modules")]
fn build_cap_registry() -> module_runner::cap_registry::CapRegistry {
    let mut reg = module_runner::cap_registry::CapRegistry::new();
    reg.register(cap_intake::IntakeModule);
    reg.register(cap_policy::PolicyModule);
    reg.register(cap_permit::PermitModule);
    reg.register(cap_llm::LlmModule);
    reg.register(cap_transport::TransportModule);
    reg.register(cap_enrich::EnrichModule);
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
        serde_json::Value::Array(a) => V::Array(
            a.iter()
                .map(json_to_nrf)
                .collect::<anyhow::Result<Vec<_>>>()?,
        ),
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

    let modules_state = Arc::new(ModulesState {
        executor,
        state_dir: state_dir.into(),
    });

    let permit_state = Arc::new(PermitState {
        store: permit_store,
    });

    (modules_state, permit_state)
}

#[cfg(feature = "modules")]
pub fn modules_router(modules_state: Arc<ModulesState>, permit_state: Arc<PermitState>) -> Router {
    let run_router = Router::new()
        .route("/modules/run", post(run_pipeline))
        .with_state(modules_state);

    run_router.merge(permit_router(permit_state))
}
