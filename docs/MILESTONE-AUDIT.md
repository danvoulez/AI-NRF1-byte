# Milestone Audit — What Already Exists (Idempotency Check)

Before implementing any milestone, check this document to avoid re-implementing
primitives that already exist at the BASE or MODULES layer.

Last updated: 2026-02-12

---

## Milestone 1 — Contract v0 (API versioning + payload shapes)

### Already implemented

| Primitive | Layer | Location | Status |
|---|---|---|---|
| `GET /version` | BASE | `services/registry/src/lib.rs` | Returns `version`, `git_sha`, `build_ts`, `modules` |
| `GET /health` | BASE | `services/registry/src/lib.rs` | Returns `{"status":"ok","modules":true}` |
| `GET /api/executions` | MODULES | `services/registry/src/routes/modules.rs` | Returns `Vec<StoredExecution>` |
| `GET /api/receipts/:cid` | MODULES | `services/registry/src/routes/modules.rs` | Returns `{execution, sirp, proofs, evidence}` |
| `GET /api/metrics` | MODULES | `services/registry/src/routes/modules.rs` | Returns `{executionsToday, ackPercentage, p99Latency, ...}` |
| `POST /modules/run` | MODULES | `services/registry/src/routes/modules.rs` | Accepts inline manifest JSON |
| TypeScript types | UI | `services/tdln-ui/lib/mock-data.ts` | `Execution`, `SIRPNode`, `Proof`, `Evidence`, etc. |
| Rust structs | MODULES | `services/registry/src/routes/modules.rs` | `StoredExecution`, `HopInfo`, `MetricEntry`, `RunResponse` |

### Still needed

- [ ] Rename `/api/*` → `/api/v0/*` (routes + UI api.ts)
- [ ] Add `GET /api/v0/whoami` (registry version + tenant + product + api_version)
- [ ] CID normalization: URL-decode path param server-side (axum does this, but test edge cases)
- [ ] Write `CONTRACT.md` documenting the 4 endpoint shapes

---

## Milestone 2 — Identity (multi-tenant / multi-product)

### Already implemented

| Primitive | Layer | Location | Status |
|---|---|---|---|
| `AuthCtx` extractor | BASE | `crates/ubl-auth/src/lib.rs` | Extracts `x-app`, `x-tenant`, `x-user-id`, `x-did` from headers |
| PoP verification | BASE | `crates/ubl-auth/src/lib.rs` | Ed25519 signature over `method\|path` via `x-signature`, `x-pubkey` |
| `require_any_role()` | BASE | `crates/ubl-auth/src/lib.rs:128` | Role-based access check |
| RBAC middleware stub | BASE | `services/registry/src/middleware/rbac.rs` | `parse_user_id()`, `require_any()` — extracts `x-user-id` as UUID |
| Bearer token gate | BASE | `services/registry/src/routes/receipts.rs:59-68` | `Authorization: Bearer <token>` check on `POST /v1/:app/:tenant/receipts` |
| `tenant` in `RunRequest` | MODULES | `services/registry/src/routes/modules.rs` | Pipeline run accepts `tenant` in body |
| `ExecutionMeta.tenant` | MODULES | `crates/modules-core/src/lib.rs:19-24` | Every capability call receives `run_id`, `tenant`, `trace_id` |
| `ExecCtx.tenant` | MODULES | `crates/module-runner/src/effects.rs:18-25` | Effect dispatch carries `tenant`, `trace_id`, `step_id` |
| Tenant in `LedgerEntry` | BASE | `crates/ubl-storage/src/ledger.rs:38` | Every ledger entry records `app` + `tenant` |
| Path-based tenant | BASE | `services/registry/src/routes/ghosts.rs` | Routes use `/:app/:tenant/ghosts` |

### Still needed

- [ ] Add `X-Product` header support to `AuthCtx` (or use body field)
- [ ] Require `X-Tenant` + `X-Product` on `/api/v0/*` routes (middleware)
- [ ] Partition `ExecutionStore` by `(tenant, product)` key
- [ ] CORS allowlist per product origin (replace `Any` with config-driven list)
- [ ] **NOTE**: `ubl-auth::AuthCtx` already does `x-app` + `x-tenant` extraction — wire it into `/api/v0/*` routes instead of re-implementing

---

## Milestone 3 — Spitter v2

### Already implemented

| Primitive | Layer | Location | Status |
|---|---|---|---|
| `spit.sh` | TOOL | `tools/product-spitter/spit.sh` | Copies mother UI, customizes `package.json`, writes `.env.local`, git init |
| `ubl product init` | CLI | `tools/ubl-cli/src/main.rs` | Hidden command, shells out to `spit.sh` |
| `--name`, `--registry`, `--out`, `--tenant` | CLI | `tools/ubl-cli/src/main.rs` | All 4 params supported |
| `.env.local` generation | TOOL | `tools/product-spitter/spit.sh` | Writes `NEXT_PUBLIC_REGISTRY_URL`, `NEXT_PUBLIC_TENANT`, `NEXT_PUBLIC_PRODUCT_NAME` |
| README generation | TOOL | `tools/product-spitter/spit.sh` | Generated with "how to run" + registry URL |

### Still needed

- [ ] Add `--product-slug` param (required, distinct from `--name`)
- [ ] Add `NEXT_PUBLIC_PRODUCT` to `.env.local` (slug, not display name)
- [ ] Branding params: `--primary-color`, `--logo-url` → inject into `tailwind.config` + layout
- [ ] Leak validation: grep output dir for `.rs`, `cap-`, `module-runner` — fail if found
- [ ] **NOTE**: The spitter already does NOT copy Rust code — it copies `services/tdln-ui/` only

---

## Milestone 4 — UX (no more curl-driven development)

### Already implemented

| Primitive | Layer | Location | Status |
|---|---|---|---|
| Run Pipeline button | UI | `services/tdln-ui/app/console/page.tsx` | Inline form: title + payload → `POST /modules/run` → toast + refresh |
| `runPipeline()` | UI | `services/tdln-ui/lib/api.ts:85-87` | Typed `RunRequest` → `RunResponse` |
| `fetchExecutions()` | UI | `services/tdln-ui/lib/api.ts:157-164` | With mock fallback |
| `fetchReceipt()` | UI | `services/tdln-ui/lib/api.ts:166-189` | Returns `{execution, sirp, proofs, evidence}` with mock fallback |
| `fetchMetrics()` | UI | `services/tdln-ui/lib/api.ts:200-206` | With mock fallback |
| `fetchAuditLog()` | UI | `services/tdln-ui/lib/api.ts:191-197` | With mock fallback |
| `fetchEvidence()` | UI | `services/tdln-ui/lib/api.ts:208-214` | With mock fallback |
| `fetchPolicies()` | UI | `services/tdln-ui/lib/api.ts:216-222` | With mock fallback |
| All 6 console pages wired | UI | `app/console/*/page.tsx` | Dashboard, executions, receipt, audits, evidence, policies |

### Still needed

- [ ] Solidify Run Pipeline form: manifest template picker (not just cap-intake)
- [ ] Receipt page: resolver links `/r/{cid}` that actually resolve
- [ ] Audits/Evidence/Policies: backend endpoints (`/api/v0/audits`, `/api/v0/evidence`, `/api/v0/policies`)
- [ ] **NOTE**: All pages already have mock fallback — they won't break if backend endpoints don't exist yet

---

## Milestone 5 — Security (auth + isolation + rate limiting)

### Already implemented

| Primitive | Layer | Location | Status |
|---|---|---|---|
| `ubl-auth::AuthCtx` | BASE | `crates/ubl-auth/src/lib.rs` | Full extractor: `x-app`, `x-tenant`, `x-did`, `x-user-id`, PoP verification |
| Ed25519 PoP | BASE | `crates/ubl-auth/src/lib.rs:40-69` | Verifies `x-signature` over `method\|path` using `x-pubkey` |
| `require_any_role()` | BASE | `crates/ubl-auth/src/lib.rs:128-135` | Role-based access control |
| RBAC middleware | BASE | `services/registry/src/middleware/rbac.rs` | `parse_user_id()` from `x-user-id` header |
| Bearer token gate | BASE | `services/registry/src/routes/receipts.rs:59-68` | On `POST /v1/:app/:tenant/receipts` |
| `IdempotentExecutor` | MODULES | `crates/module-runner/src/effects.rs:75-132` | Deduplicates effects by `blake3(capsule_id \|\| step_id \|\| effect_kind)` |
| `tower-http` dep | BASE | `services/registry/Cargo.toml:31` | `cors` + `trace` features enabled |

### Still needed

- [ ] Wire `ubl-auth::AuthCtx` into `/api/v0/*` routes (it exists but isn't used there)
- [ ] API key per product: simple token lookup (can extend `AuthCtx` or add middleware)
- [ ] Tenant isolation: product A's key scoped to its tenant only
- [ ] Rate limiting: `tower::limit::RateLimitLayer` or custom per-product limiter
- [ ] **NOTE**: PoP (Ed25519 signature) is already the strongest auth primitive — API keys are a simpler alternative for product children. Consider supporting BOTH: PoP for internal, API key for external products.

---

## Milestone 6 — Ops (persistence + observability + deploy)

### Already implemented

| Primitive | Layer | Location | Status |
|---|---|---|---|
| `LedgerWriter` trait | BASE | `crates/ubl-storage/src/ledger.rs:123-126` | `async fn append(&self, entry: &LedgerEntry)` |
| `LedgerEntry` struct | BASE | `crates/ubl-storage/src/ledger.rs:34-46` | Full audit entry: `ts`, `event`, `app`, `tenant`, `user_id`, `roles`, `entity_id`, `cid`, `did`, `decision`, `payload` |
| `NullLedger` | BASE | `crates/ubl-storage/src/ledger.rs:132-139` | No-op fallback when no ledger module compiled |
| `LedgerEvent` enum | BASE | `crates/ubl-storage/src/ledger.rs:23-30` | `ReceiptCreated`, `GhostCreated`, `GhostPromoted`, `GhostExpired` |
| Ghost routes use ledger | BASE | `services/registry/src/routes/ghosts.rs:72-89` | `state.ledger.append(&entry).await` on create/promote/expire |
| `ubl-model::Receipt` | BASE | `crates/ubl-model/src/lib.rs:7-21` | Postgres-backed `Receipt` struct with `sqlx::FromRow` |
| `upsert_receipt()` | BASE | `crates/ubl-model/src/lib.rs:37-61` | Postgres upsert with `ON CONFLICT (tenant_id, cid)` |
| S3 storage (feature-gated) | BASE | `crates/ubl-storage/src/lib.rs:3-6` | `S3Store` behind `s3` feature flag |
| `tracing` structured logs | BOTH | `services/registry/src/main.rs`, `crates/module-runner/src/runner.rs` | `run_id`, `step_id`, `kind`, `verdict`, `elapsed_ms` in every log |
| `ExecutionMeta.trace_id` | MODULES | `crates/modules-core/src/lib.rs:22` | Propagated through every capability call |
| `ExecCtx.trace_id` | MODULES | `crates/module-runner/src/effects.rs:20` | Propagated through every effect dispatch |
| Per-step metrics | MODULES | `crates/module-runner/src/runner.rs:122-125` | `step_metrics: Vec<(step_id, key, value)>` including `duration_ms` |
| `tower-http` trace feature | BASE | `services/registry/Cargo.toml:31` | `trace` feature enabled (not yet wired as middleware) |
| `/health`, `/healthz`, `/readyz` | BASE | `services/registry/src/lib.rs:47-50` | Health check endpoints for load balancers |

### Still needed

- [ ] Wire `LedgerWriter` into `/api/v0/*` routes (currently only ghosts use it)
- [ ] Add `PipelineExecuted` event to `LedgerEvent` enum
- [ ] Replace in-memory `VecDeque` with ledger-backed query (or Postgres via `ubl-model`)
- [ ] Wire `tower-http::trace::TraceLayer` into router (dep exists, not applied)
- [ ] Request ID middleware (generate UUID per request, propagate in logs)
- [ ] Deploy: TLS reverse proxy config, Dockerfile, docker-compose
- [ ] **NOTE**: `ubl-model` already has a full Postgres `Receipt` model with upsert — this is the path to persistent storage. The `LedgerWriter` trait is the append-only audit trail that runs in parallel.

---

## Summary: What's reusable vs what's new

| Area | BASE has | MODULES has | New work needed |
|---|---|---|---|
| **Auth** | `ubl-auth::AuthCtx` (PoP, x-tenant, x-app, roles) | — | Wire into `/api/v0/*`, add API key option |
| **RBAC** | `middleware/rbac.rs` (parse_user_id, require_any) | — | Apply to product routes |
| **Tenant** | `LedgerEntry.tenant`, path params `/:app/:tenant/*` | `ExecutionMeta.tenant`, `ExecCtx.tenant` | Partition store, require headers |
| **Ledger** | `LedgerWriter` trait, `NullLedger`, `LedgerEntry` | — | Add `PipelineExecuted` event, wire to `/api/v0/*` |
| **Postgres** | `ubl-model::Receipt` with `upsert_receipt()` | — | Wire as persistent backend for executions |
| **Tracing** | `tracing` + `tracing-subscriber` + `tower-http[trace]` | `run_id`, `trace_id`, `step_metrics` | Wire `TraceLayer`, add request ID middleware |
| **Rate limit** | `tower-http` dep exists | — | Add `RateLimitLayer` per product |
| **Idempotency** | — | `IdempotentExecutor` (effect dedup) | Already done for effects |
| **CID** | `blake3` everywhere, `b3:` prefix convention | `hop_payload_id()` | Normalize URL encoding |
| **S3** | `ubl-storage::S3Store` (feature-gated) | — | Enable for production artifacts |
