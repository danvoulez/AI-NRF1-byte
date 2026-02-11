//! Integration tests for DispatchExecutor with real adapters wired via builder.
//!
//! These tests verify the full dispatch path: signer, LLM, permit store,
//! and the resume watcher — all without network (no `live` feature needed).

use module_runner::adapters::llm::StubProvider;
use module_runner::adapters::permit::{PermitStore, TicketStatus};
use module_runner::adapters::resume::{check_resumable, ResumeJob, ResumeStore};
use module_runner::adapters::signer::NoopSigner;
use module_runner::effects::{DispatchExecutor, EffectExecutor, ExecCtx};
use modules_core::Effect;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "ai-nrf1-dispatch-integ-{}-{:?}",
        name,
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn make_ctx(tenant: &str, step: &str) -> ExecCtx {
    ExecCtx {
        tenant: tenant.into(),
        trace_id: "trace-001".into(),
        io_bindings: serde_json::json!({
            "webhook.url": "https://example.com/hook",
            "webhook.hmac": "secret123",
            "NODE_KEY": "test-node-key",
            "partner.relay.url": "https://relay.example.com",
            "OPENAI_GPT4O_MINI": "test-model-key",
        }),
        now_nanos: 1_000_000_000,
        step_id: step.into(),
        capsule_id_hex: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".into(),
    }
}

// ---------------------------------------------------------------------------
// DispatchExecutor with NoopSigner
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_append_receipt_with_noop_signer() {
    let dir = temp_dir("signer");
    let executor = DispatchExecutor::builder(dir.to_str().unwrap())
        .signer(NoopSigner)
        .build();

    let effect = Effect::AppendReceipt {
        payload_nrf: vec![1, 2, 3, 4],
        signer_binding: "NODE_KEY".into(),
    };

    let ctx = make_ctx("acme", "step-transport");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn dispatch_append_receipt_without_signer_still_ok() {
    let dir = temp_dir("no_signer");
    let executor = DispatchExecutor::new(dir.to_str().unwrap());

    let effect = Effect::AppendReceipt {
        payload_nrf: vec![1, 2, 3],
        signer_binding: "NODE_KEY".into(),
    };

    let ctx = make_ctx("acme", "step-transport");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok(), "should log-only when no signer configured");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// DispatchExecutor with StubProvider (LLM)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_invoke_llm_with_stub_provider() {
    let dir = temp_dir("llm");
    let executor = DispatchExecutor::builder(dir.to_str().unwrap())
        .llm(StubProvider {
            response: "ALLOW — risk acceptable".into(),
        })
        .build();

    let effect = Effect::InvokeLlm {
        model_binding: "OPENAI_GPT4O_MINI".into(),
        prompt: "Evaluate risk for amount=5000".into(),
        max_tokens: 100,
        cache_key: Some("test-cache-key".into()),
    };

    let ctx = make_ctx("acme", "step-llm");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn dispatch_invoke_llm_without_provider_still_ok() {
    let dir = temp_dir("no_llm");
    let executor = DispatchExecutor::new(dir.to_str().unwrap());

    let effect = Effect::InvokeLlm {
        model_binding: "OPENAI_GPT4O_MINI".into(),
        prompt: "test".into(),
        max_tokens: 50,
        cache_key: None,
    };

    let ctx = make_ctx("acme", "step-llm");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok(), "should log-only when no LLM configured");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// DispatchExecutor: Webhook and RelayOut (no http adapter = log-only)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_webhook_without_http_adapter() {
    let dir = temp_dir("webhook_stub");
    let executor = DispatchExecutor::new(dir.to_str().unwrap());

    let effect = Effect::Webhook {
        url: "webhook.url".into(),
        body: b"test body".to_vec(),
        content_type: "application/json".into(),
        hmac_key_env: Some("webhook.hmac".into()),
    };

    let ctx = make_ctx("acme", "step-enrich");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn dispatch_relay_out_without_http_adapter() {
    let dir = temp_dir("relay_stub");
    let executor = DispatchExecutor::new(dir.to_str().unwrap());

    let effect = Effect::RelayOut {
        to: "partner-a".into(),
        url_binding: "partner.relay.url".into(),
        body: vec![0xDE, 0xAD],
    };

    let ctx = make_ctx("acme", "step-transport");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// DispatchExecutor: WriteStorage
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_write_storage() {
    let dir = temp_dir("storage");
    let executor = DispatchExecutor::new(dir.to_str().unwrap());

    let effect = Effect::WriteStorage {
        path: "artifacts/status.html".into(),
        bytes: b"<h1>OK</h1>".to_vec(),
        mime: "text/html".into(),
    };

    let ctx = make_ctx("acme", "step-enrich");
    let result = executor.execute(&effect, &ctx).await;
    assert!(result.is_ok());

    let written = std::fs::read_to_string(dir.join("artifacts/status.html")).unwrap();
    assert_eq!(written, "<h1>OK</h1>");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// DispatchExecutor: Consent queue + close
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_consent_queue_and_close() {
    let dir = temp_dir("consent");
    let executor = DispatchExecutor::new(dir.to_str().unwrap());
    let ctx = make_ctx("acme", "step-permit");

    // Queue
    let queue_effect = Effect::QueueConsentTicket {
        ticket_id: "t-integ-001".into(),
        expires_at: i64::MAX,
        required_roles: vec!["ops".into(), "risk".into()],
        k: 2,
        n: 2,
    };
    executor.execute(&queue_effect, &ctx).await.unwrap();

    // Verify file exists
    let ticket_path = dir.join("permit-tickets/acme/t-integ-001.json");
    assert!(ticket_path.exists());

    // Close
    let close_effect = Effect::CloseConsentTicket {
        ticket_id: "t-integ-001".into(),
        outcome: "ALLOW".into(),
    };
    executor.execute(&close_effect, &ctx).await.unwrap();

    let raw = std::fs::read_to_string(&ticket_path).unwrap();
    let ticket: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(ticket["status"], "ALLOW");

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// DispatchExecutor with PermitStore: consent queue writes typed Ticket
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_consent_with_permit_store() {
    let dir = temp_dir("permit_store");
    let permit_store = PermitStore::new(dir.to_str().unwrap());
    let executor = DispatchExecutor::builder(dir.to_str().unwrap())
        .permit_store(permit_store)
        .build();

    let ctx = make_ctx("acme", "step-permit");
    let queue_effect = Effect::QueueConsentTicket {
        ticket_id: "t-typed-001".into(),
        expires_at: i64::MAX,
        required_roles: vec!["ops".into()],
        k: 1,
        n: 1,
    };
    executor.execute(&queue_effect, &ctx).await.unwrap();

    // Read back via PermitStore
    let ps = PermitStore::new(dir.to_str().unwrap());
    let ticket = ps.get("acme", "t-typed-001").unwrap().unwrap();
    assert_eq!(ticket.status, TicketStatus::Pending);
    assert_eq!(ticket.k, 1);

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Full builder: signer + LLM + permit store together
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_full_builder() {
    let dir = temp_dir("full_builder");
    let permit_store = PermitStore::new(dir.to_str().unwrap());
    let executor = DispatchExecutor::builder(dir.to_str().unwrap())
        .signer(NoopSigner)
        .llm(StubProvider {
            response: "ALLOW".into(),
        })
        .permit_store(permit_store)
        .build();

    let ctx = make_ctx("acme", "step-all");

    // LLM
    executor
        .execute(
            &Effect::InvokeLlm {
                model_binding: "OPENAI_GPT4O_MINI".into(),
                prompt: "test".into(),
                max_tokens: 10,
                cache_key: None,
            },
            &ctx,
        )
        .await
        .unwrap();

    // Signer
    executor
        .execute(
            &Effect::AppendReceipt {
                payload_nrf: vec![1, 2, 3],
                signer_binding: "NODE_KEY".into(),
            },
            &ctx,
        )
        .await
        .unwrap();

    // Storage
    executor
        .execute(
            &Effect::WriteStorage {
                path: "test.txt".into(),
                bytes: b"hello".to_vec(),
                mime: "text/plain".into(),
            },
            &ctx,
        )
        .await
        .unwrap();

    // Consent
    executor
        .execute(
            &Effect::QueueConsentTicket {
                ticket_id: "t-full".into(),
                expires_at: i64::MAX,
                required_roles: vec!["ops".into()],
                k: 1,
                n: 1,
            },
            &ctx,
        )
        .await
        .unwrap();

    // Verify all side effects
    assert!(dir.join("test.txt").exists());
    assert!(dir.join("permit-tickets/acme/t-full.json").exists());

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Resume watcher integration: queue → approve → check_resumable
// ---------------------------------------------------------------------------

#[tokio::test]
async fn resume_after_permit_allow() {
    let dir = temp_dir("resume_flow");
    let permit_store = PermitStore::new(dir.to_str().unwrap());
    let resume_store = ResumeStore::new(dir.to_str().unwrap());
    let executor = DispatchExecutor::builder(dir.to_str().unwrap())
        .permit_store(PermitStore::new(dir.to_str().unwrap()))
        .build();

    let ctx = make_ctx("acme", "step-permit");

    // 1. Queue consent ticket via executor
    executor
        .execute(
            &Effect::QueueConsentTicket {
                ticket_id: "t-resume".into(),
                expires_at: i64::MAX,
                required_roles: vec!["ops".into()],
                k: 1,
                n: 1,
            },
            &ctx,
        )
        .await
        .unwrap();

    // 2. Save resume job
    let job = ResumeJob {
        job_id: "j-resume".into(),
        tenant: "acme".into(),
        ticket_id: "t-resume".into(),
        trace_id: "trace-001".into(),
        capsule_id_hex: "abcd".into(),
        resume_after_step: 2,
        env_json: serde_json::json!({"amount": 5000}),
        manifest_name: "test-product".into(),
        receipts_hex: vec![],
        created_at: 1_000_000,
        completed: false,
    };
    resume_store.save(&job).unwrap();

    // 3. Not yet resumable (ticket is PENDING)
    let ready = check_resumable(&resume_store, &permit_store, "acme").unwrap();
    assert_eq!(ready.len(), 0);

    // 4. Approve the ticket
    let outcome = permit_store
        .approve("acme", "t-resume", "ops", 2_000_000, None)
        .unwrap();
    assert!(matches!(
        outcome,
        module_runner::adapters::permit::PermitOutcome::Closed(TicketStatus::Allow)
    ));

    // 5. Now resumable
    let ready = check_resumable(&resume_store, &permit_store, "acme").unwrap();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].job_id, "j-resume");

    // 6. Mark completed
    resume_store.mark_completed("acme", "j-resume").unwrap();
    let ready = check_resumable(&resume_store, &permit_store, "acme").unwrap();
    assert_eq!(ready.len(), 0);

    let _ = std::fs::remove_dir_all(&dir);
}
