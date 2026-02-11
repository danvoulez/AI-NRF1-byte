//! End-to-end pipeline tests for the module runtime.
//!
//! These tests wire real capability modules into the Runner and verify
//! the full cycle: env flow, verdicts, effects, artifacts, and flow control.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use module_runner::assets::MemoryResolver;
use module_runner::cap_registry::CapRegistry;
use module_runner::effects::{EffectExecutor, ExecCtx, NoopExecutor};
use module_runner::manifest::Manifest;
use module_runner::runner::Runner;
use modules_core::{Effect, Verdict};

// ---------------------------------------------------------------------------
// RecordingExecutor — captures effects for assertions
// ---------------------------------------------------------------------------

struct RecordingExecutor {
    effects: Arc<Mutex<Vec<String>>>,
}

impl RecordingExecutor {
    fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
        let log = Arc::new(Mutex::new(vec![]));
        (Self { effects: log.clone() }, log)
    }
}

#[async_trait::async_trait]
impl EffectExecutor for RecordingExecutor {
    async fn execute(&self, effect: &Effect, ctx: &ExecCtx) -> anyhow::Result<()> {
        let kind = match effect {
            Effect::Webhook { .. } => "webhook",
            Effect::WriteStorage { .. } => "write_storage",
            Effect::QueueConsentTicket { .. } => "consent_queue",
            Effect::CloseConsentTicket { .. } => "consent_close",
            Effect::AppendReceipt { .. } => "append_receipt",
            Effect::RelayOut { .. } => "relay_out",
            Effect::InvokeLlm { .. } => "invoke_llm",
        };
        self.effects
            .lock()
            .unwrap()
            .push(format!("{}:{}", ctx.step_id, kind));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_env() -> nrf1::Value {
    let mut m = BTreeMap::new();
    let mut req = BTreeMap::new();
    let mut body = BTreeMap::new();
    let mut user = BTreeMap::new();
    user.insert("id".into(), nrf1::Value::String("user-42".into()));
    body.insert("user".into(), nrf1::Value::Map(user));
    body.insert("score_scaled".into(), nrf1::Value::Int(800));
    req.insert("body".into(), nrf1::Value::Map(body));
    m.insert("req".into(), nrf1::Value::Map(req));
    nrf1::Value::Map(m)
}

fn bindings() -> serde_json::Value {
    serde_json::json!({
        "webhook.url": "https://example.com/hooks",
        "WH_SEC": "test-secret",
        "status.dir": "/tmp/test-status"
    })
}

// ---------------------------------------------------------------------------
// Test 1: Basic pipeline — intake → policy(ALLOW) → enrich
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_basic_allow_pipeline() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "v": "product-v1",
        "name": "test-basic",
        "version": "1.0.0",
        "pipeline": [
            {
                "step_id": "normalize",
                "kind": "cap-intake",
                "version": "^1",
                "config": {
                    "mapping": [
                        { "from": "req.body.user.id", "to": "ctx.user.id" },
                        { "from": "req.body.score_scaled", "to": "decision.metrics.risk_score" }
                    ],
                    "defaults": { "ctx.kind": "transaction" }
                }
            },
            {
                "step_id": "policy",
                "kind": "cap-policy",
                "version": "^1",
                "config": {
                    "decision_on_fail": "DENY",
                    "rules": [
                        { "kind": "EXIST", "paths": ["ctx.user.id", "ctx.kind"] },
                        { "kind": "THRESHOLD_RANGE", "path": "decision.metrics.risk_score", "min": 700, "max": 900 }
                    ]
                }
            },
            {
                "step_id": "enrich",
                "kind": "cap-enrich",
                "version": "^1",
                "config": {
                    "drivers": [{ "kind": "status-page" }, { "kind": "webhook" }],
                    "redaction": [],
                    "webhook_binding": "WH_SEC"
                }
            }
        ]
    }))
    .unwrap();

    let mut caps = CapRegistry::new();
    caps.register(cap_intake::IntakeModule);
    caps.register(cap_policy::PolicyModule);
    caps.register(cap_enrich::EnrichModule);

    let (executor, log) = RecordingExecutor::new();
    let runner = Runner::new(
        &caps,
        Box::new(MemoryResolver::new()),
        &executor,
        bindings(),
        "test-tenant",
    );

    let result = runner.run(&manifest, make_env()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Allow);
    assert!(result.stopped_at.is_none(), "should complete all steps");
    assert_eq!(result.receipts.len(), 3);
    assert!(!result.artifacts.is_empty(), "enrich should produce artifacts");

    let effects = log.lock().unwrap();
    assert!(
        effects.iter().any(|e| e.starts_with("enrich:write_storage")),
        "enrich should emit write_storage"
    );
    assert!(
        effects.iter().any(|e| e.starts_with("enrich:webhook")),
        "enrich should emit webhook"
    );
}

// ---------------------------------------------------------------------------
// Test 2: Policy DENY halts pipeline
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_policy_deny_halts() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "v": "product-v1",
        "name": "test-deny",
        "version": "1.0.0",
        "pipeline": [
            {
                "step_id": "normalize",
                "kind": "cap-intake",
                "version": "^1",
                "config": {
                    "mapping": [
                        { "from": "req.body.user.id", "to": "ctx.user.id" }
                    ]
                }
            },
            {
                "step_id": "policy",
                "kind": "cap-policy",
                "version": "^1",
                "config": {
                    "decision_on_fail": "DENY",
                    "rules": [
                        { "kind": "EXIST", "paths": ["ctx.NONEXISTENT_FIELD"] }
                    ]
                }
            },
            {
                "step_id": "enrich",
                "kind": "cap-enrich",
                "version": "^1",
                "config": { "drivers": [] }
            }
        ]
    }))
    .unwrap();

    let mut caps = CapRegistry::new();
    caps.register(cap_intake::IntakeModule);
    caps.register(cap_policy::PolicyModule);
    caps.register(cap_enrich::EnrichModule);

    let runner = Runner::new(
        &caps,
        Box::new(MemoryResolver::new()),
        &NoopExecutor,
        bindings(),
        "test-tenant",
    );

    let result = runner.run(&manifest, make_env()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Deny);
    assert_eq!(result.stopped_at.as_deref(), Some("policy"));
    assert_eq!(result.receipts.len(), 2, "only normalize + policy ran");
}

// ---------------------------------------------------------------------------
// Test 3: Policy REQUIRE triggers consent, halts pipeline
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_policy_require_triggers_consent() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "v": "product-v1",
        "name": "test-require",
        "version": "1.0.0",
        "pipeline": [
            {
                "step_id": "normalize",
                "kind": "cap-intake",
                "version": "^1",
                "config": {
                    "mapping": [
                        { "from": "req.body.user.id", "to": "ctx.user.id" }
                    ]
                }
            },
            {
                "step_id": "policy",
                "kind": "cap-policy",
                "version": "^1",
                "config": {
                    "decision_on_fail": "REQUIRE",
                    "rules": [
                        { "kind": "EXIST", "paths": ["ctx.NONEXISTENT_FIELD"] }
                    ]
                }
            },
            {
                "step_id": "permit",
                "kind": "cap-permit",
                "version": "^1",
                "config": {
                    "quorum": { "k": 1, "n": 2, "roles": ["ops", "legal"] },
                    "ttl_sec": 1800,
                    "timeout_action": "DENY"
                }
            },
            {
                "step_id": "enrich",
                "kind": "cap-enrich",
                "version": "^1",
                "config": { "drivers": [] }
            }
        ]
    }))
    .unwrap();

    let mut caps = CapRegistry::new();
    caps.register(cap_intake::IntakeModule);
    caps.register(cap_policy::PolicyModule);
    caps.register(cap_permit::PermitModule);
    caps.register(cap_enrich::EnrichModule);

    let (executor, log) = RecordingExecutor::new();
    let runner = Runner::new(
        &caps,
        Box::new(MemoryResolver::new()),
        &executor,
        bindings(),
        "test-tenant",
    );

    let result = runner.run(&manifest, make_env()).await.unwrap();

    // Policy returns REQUIRE, pipeline halts at policy step
    assert_eq!(result.verdict, Verdict::Require);
    assert_eq!(result.stopped_at.as_deref(), Some("policy"));
    assert_eq!(result.receipts.len(), 2, "only normalize + policy ran");

    // No consent effects yet (permit step didn't run because pipeline halted at policy)
    let effects = log.lock().unwrap();
    assert!(
        !effects.iter().any(|e| e.contains("consent_queue")),
        "permit step should NOT have run (pipeline halted at policy REQUIRE)"
    );
}

// ---------------------------------------------------------------------------
// Test 4: cap-llm never sets verdict
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_llm_never_verdict() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "v": "product-v1",
        "name": "test-llm",
        "version": "1.0.0",
        "pipeline": [
            {
                "step_id": "assist",
                "kind": "cap-llm",
                "version": "^0",
                "config": {
                    "model_binding": "TEST_MODEL",
                    "prompt_cid": "b3:0000000000000000000000000000000000000000000000000000000000000000",
                    "inputs": {},
                    "produce": ["artifact:json:analysis"]
                }
            }
        ]
    }))
    .unwrap();

    let mut caps = CapRegistry::new();
    caps.register(cap_llm::LlmModule);

    let (executor, log) = RecordingExecutor::new();
    let runner = Runner::new(
        &caps,
        Box::new(MemoryResolver::new()),
        &executor,
        bindings(),
        "test-tenant",
    );

    let result = runner.run(&manifest, make_env()).await.unwrap();

    // Verdict stays Allow (default) — cap-llm never sets it
    assert_eq!(result.verdict, Verdict::Allow);
    assert!(result.stopped_at.is_none());
    assert!(!result.artifacts.is_empty(), "should produce JSON artifact");

    let effects = log.lock().unwrap();
    assert!(
        effects.iter().any(|e| e.contains("invoke_llm")),
        "should emit InvokeLlm effect"
    );
}

// ---------------------------------------------------------------------------
// Test 5: Transport produces receipt + relay effects
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_transport_produces_effects() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "v": "product-v1",
        "name": "test-transport",
        "version": "1.0.0",
        "pipeline": [
            {
                "step_id": "transport",
                "kind": "cap-transport",
                "version": "^1",
                "config": {
                    "node": "did:ubl:node-01#key-1",
                    "relay": [{ "kind": "http", "url": "https://relay.example.com" }]
                }
            }
        ]
    }))
    .unwrap();

    let mut caps = CapRegistry::new();
    caps.register(cap_transport::TransportModule);

    let (executor, log) = RecordingExecutor::new();
    let runner = Runner::new(
        &caps,
        Box::new(MemoryResolver::new()),
        &executor,
        bindings(),
        "test-tenant",
    );

    let result = runner.run(&manifest, make_env()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Allow);
    assert_eq!(result.receipts.len(), 1);

    let effects = log.lock().unwrap();
    assert!(effects.iter().any(|e| e.contains("append_receipt")));
    assert!(effects.iter().any(|e| e.contains("relay_out")));
}

// ---------------------------------------------------------------------------
// Test 6: Full 6-step pipeline
// ---------------------------------------------------------------------------

#[tokio::test]
async fn e2e_full_6_step_pipeline_allow() {
    let manifest: Manifest = serde_json::from_value(serde_json::json!({
        "v": "product-v1",
        "name": "api-receipt-gateway",
        "version": "2.0.0",
        "pipeline": [
            {
                "step_id": "normalize",
                "kind": "cap-intake",
                "version": "^1",
                "config": {
                    "mapping": [
                        { "from": "req.body.user.id", "to": "ctx.user.id" },
                        { "from": "req.body.score_scaled", "to": "decision.metrics.risk_score" }
                    ],
                    "defaults": { "ctx.kind": "transaction" }
                }
            },
            {
                "step_id": "policy",
                "kind": "cap-policy",
                "version": "^1",
                "config": {
                    "decision_on_fail": "DENY",
                    "rules": [
                        { "kind": "EXIST", "paths": ["ctx.user.id"] },
                        { "kind": "THRESHOLD_RANGE", "path": "decision.metrics.risk_score", "min": 700, "max": 900 }
                    ]
                }
            },
            {
                "step_id": "assist",
                "kind": "cap-llm",
                "version": "^0",
                "config": {
                    "model_binding": "TEST_MODEL",
                    "prompt_cid": "b3:0000000000000000000000000000000000000000000000000000000000000000",
                    "inputs": {},
                    "produce": ["artifact:json:analysis"]
                }
            },
            {
                "step_id": "transport",
                "kind": "cap-transport",
                "version": "^1",
                "config": {
                    "node": "did:ubl:node-01#key-1",
                    "relay": [{ "kind": "http", "url": "https://relay.example.com" }]
                }
            },
            {
                "step_id": "enrich",
                "kind": "cap-enrich",
                "version": "^1",
                "config": {
                    "drivers": [{ "kind": "status-page" }, { "kind": "webhook" }],
                    "redaction": [],
                    "webhook_binding": "WH_SEC"
                }
            }
        ]
    }))
    .unwrap();

    let mut caps = CapRegistry::new();
    caps.register(cap_intake::IntakeModule);
    caps.register(cap_policy::PolicyModule);
    caps.register(cap_llm::LlmModule);
    caps.register(cap_transport::TransportModule);
    caps.register(cap_enrich::EnrichModule);

    let (executor, log) = RecordingExecutor::new();
    let runner = Runner::new(
        &caps,
        Box::new(MemoryResolver::new()),
        &executor,
        bindings(),
        "test-tenant",
    );

    let result = runner.run(&manifest, make_env()).await.unwrap();

    assert_eq!(result.verdict, Verdict::Allow);
    assert!(result.stopped_at.is_none(), "all steps should complete");
    assert_eq!(result.receipts.len(), 5, "5 steps = 5 hop receipts");
    assert!(result.artifacts.len() >= 2, "llm + enrich artifacts");

    let effects = log.lock().unwrap();
    assert!(effects.iter().any(|e| e.starts_with("assist:invoke_llm")));
    assert!(effects.iter().any(|e| e.starts_with("transport:append_receipt")));
    assert!(effects.iter().any(|e| e.starts_with("transport:relay_out")));
    assert!(effects.iter().any(|e| e.starts_with("enrich:write_storage")));
    assert!(effects.iter().any(|e| e.starts_with("enrich:webhook")));
}
