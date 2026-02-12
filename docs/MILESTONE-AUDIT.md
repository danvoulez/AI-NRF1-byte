# Milestone Audit — Implementation Status

Tracks what was implemented for each roadmap milestone.

Last updated: 2026-02-12

---

## Milestone 1 — Contract v0 ✅ DONE

| What | Where | Commit |
|---|---|---|
| Routes renamed `/api/*` → `/api/v0/*` | `services/registry/src/routes/modules.rs` | feat(M1) |
| `/modules/run` → `/api/v0/run` | `services/registry/src/routes/modules.rs` | feat(M1) |
| `GET /api/v0/whoami` | `services/registry/src/lib.rs` | feat(M1) |
| CID URL-decode via `urlencoding` crate | `services/registry/src/routes/modules.rs` | feat(M1) |
| `docs/CONTRACT.md` — full endpoint docs | `docs/CONTRACT.md` | feat(M1) |
| Mother + template `api.ts` → `/api/v0/*` | `services/tdln-ui/lib/api.ts`, `services/ui-template/lib/api.ts` | feat(M1) |

---

## Milestone 2 — Identity ✅ DONE

| What | Where | Commit |
|---|---|---|
| `middleware/identity.rs` — `require_identity` | `services/registry/src/middleware/identity.rs` | feat(M2) |
| `ProductIdentity` struct in extensions | `services/registry/src/middleware/identity.rs` | feat(M2) |
| `ExecutionStore` partitioned by `(tenant, product)` | `services/registry/src/routes/modules.rs` | feat(M2) |
| All handlers scoped to caller's partition | `services/registry/src/routes/modules.rs` | feat(M2) |
| `X-Tenant` + `X-Product` headers sent by UI | `services/tdln-ui/lib/api.ts`, `services/ui-template/lib/api.ts` | feat(M2) |
| `TENANT` + `PRODUCT` env vars in `env.ts` | `services/tdln-ui/lib/env.ts`, `services/ui-template/lib/env.ts` | feat(M2) |

---

## Milestone 3 — Spitter v2 ✅ DONE

| What | Where | Commit |
|---|---|---|
| `services/ui-template/` — vanilla Next.js skeleton | `services/ui-template/` | feat(template) |
| `product.json` manifest schema | `services/ui-template/product.json` | feat(template) |
| `lib/product.ts` — `ProductConfig`, `hasPage()`, `hasFeature()` | `services/ui-template/lib/product.ts` | feat(template) |
| `layout.tsx` + `app-sidebar.tsx` read from `product.json` | `services/ui-template/app/layout.tsx`, `components/console/app-sidebar.tsx` | feat(template) |
| `spit.sh` rewritten — copies template, generates `product.json` | `tools/product-spitter/spit.sh` | feat(template) |
| New params: `--slug`, `--locale`, `--primary`, `--accent` | `tools/product-spitter/spit.sh` | feat(template) |
| Step 6: leak validation (`.rs`, `Cargo.toml`, `cap-*`) | `tools/product-spitter/spit.sh` | feat(template) |
| Mother gets `product.json` for consistency | `services/tdln-ui/product.json` | feat(template) |

---

## Milestone 4 — UX ✅ DONE

| What | Where | Commit |
|---|---|---|
| `GET /api/v0/audits` — audit log from executions | `services/registry/src/routes/modules.rs` | feat(M4) |
| `GET /api/v0/evidence` — evidence from receipt chains | `services/registry/src/routes/modules.rs` | feat(M4) |
| `GET /api/v0/policies` — active policy packs (stub) | `services/registry/src/routes/modules.rs` | feat(M4) |
| `GET /r/:cid` — resolver redirect → `/console/r/:cid` | `services/registry/src/routes/modules.rs` | feat(M4) |
| `CONTRACT.md` updated with all new endpoints | `docs/CONTRACT.md` | feat(M4) |

---

## Milestone 5 — Security ✅ DONE

| What | Where | Commit |
|---|---|---|
| `middleware/api_key.rs` — `ApiKeyStore` from `API_KEYS` env | `services/registry/src/middleware/api_key.rs` | feat(M5) |
| Dev mode: `API_KEYS` not set → all requests pass | `services/registry/src/middleware/api_key.rs` | feat(M5) |
| `X-API-Key` validation per product | `services/registry/src/middleware/api_key.rs` | feat(M5) |
| `middleware/rate_limit.rs` — per-product token bucket | `services/registry/src/middleware/rate_limit.rs` | feat(M5) |
| `RATE_LIMIT_RPM` env var (default 120, 0 to disable) | `services/registry/src/middleware/rate_limit.rs` | feat(M5) |
| Middleware stack: identity → api_key → rate_limit | `services/registry/src/routes/modules.rs` | feat(M5) |

---

## Milestone 6 — Ops ✅ DONE (phase 1)

| What | Where | Commit |
|---|---|---|
| `TraceLayer::new_for_http()` wired into router | `services/registry/src/lib.rs` | feat(M6) |
| Structured request logging for all routes | `services/registry/src/lib.rs` | feat(M6) |

### Still available for phase 2

- [ ] Wire `LedgerWriter` into `/api/v0/*` routes (currently only ghosts use it)
- [ ] Add `PipelineExecuted` event to `LedgerEvent` enum
- [ ] Replace in-memory `VecDeque` with Postgres via `ubl-model`
- [ ] Request ID middleware (generate UUID per request, propagate in logs)
- [ ] Deploy: Dockerfile, docker-compose, TLS reverse proxy config
- **NOTE**: `ubl-model` already has Postgres `Receipt` model with upsert. `LedgerWriter` trait is the append-only audit trail.

---

## Middleware stack (execution order on `/api/v0/*`)

```
Request
  │
  ├─ 1. require_identity   → extract X-Tenant + X-Product (400 if missing)
  ├─ 2. require_api_key    → validate X-API-Key (401 if invalid, skip in dev)
  ├─ 3. rate_limit         → per-product token bucket (429 if exhausted)
  │
  └─ Handler (run_pipeline, list_executions, get_receipt, etc.)
```

## Environment variables

| Variable | Default | Description |
|---|---|---|
| `NEXT_PUBLIC_REGISTRY_URL` | `http://localhost:4000` | Registry base URL |
| `NEXT_PUBLIC_TENANT` | `default` | Tenant slug sent as `X-Tenant` |
| `NEXT_PUBLIC_PRODUCT` | product-specific | Product slug sent as `X-Product` |
| `API_KEYS` | (unset = dev mode) | `product1:sk_key1,product2:sk_key2` |
| `RATE_LIMIT_RPM` | `120` | Requests per minute per product (0 = disabled) |
| `STATE_DIR` | `~/.ai-nrf1/state` | Module runner state directory |
