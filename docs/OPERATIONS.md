# Guia Oficial de Operação — BASE (ai-nrf1 / ubl)

Este documento padroniza **como trabalhar**, **testar**, **versionar** e **publicar binários** da BASE.
O foco é: PRs → CI → release/tag → artefatos (Releases + LAB512/local) → verificação offline (WASM) → supply chain (SBOM + checksums + assinatura).

## 1) Fluxo de branches e PRs

- `main`: deve ficar sempre verde.
- Branches:
  - `feat/<slug>`, `fix/<slug>`, `chore/<slug>`
- Commits: recomendado usar Conventional Commits (ex.: `feat(capsule): ...`, `fix(nrf): ...`).
- Antes de abrir PR:
  ```bash
  git fetch origin
  git rebase origin/main
  ```

### Proteções recomendadas (GitHub UI)

Em **Settings → Branches → Branch protection rules** para `main`:
- Require status checks (CI) to pass before merging
- Require at least 1 approval
- Disallow force pushes

Observação: isso é configuração do repositório (não dá para “commitar” no git).

## 2) O que o CI executa (qualidade)

### 2.1 Rust (formatação, lint, testes)

```bash
cargo fmt --all -- --check
cargo clippy -p nrf-core -p ai-nrf1 -p ubl_json_view -p ubl_capsule --all-targets --all-features -- -D warnings
cargo test --workspace --locked
```

### 2.2 Vetores (KATs) e invariantes (capsule/receipts/expired/tamper)

```bash
make vectors-verify
```

### 2.3 WASM (offline verify smoke via Node)

Requer `wasm-pack` e `node`:

```bash
PATH="$HOME/.cargo/bin:$PATH" cargo install wasm-pack
PATH="$HOME/.cargo/bin:$PATH" bash tools/wasm/build_node_pkgs.sh
node tests/wasm/node_smoke.cjs
```

### 2.4 Diferencial Python ↔ Rust

```bash
python3 -m pip install -r tests/differential/requirements.txt
cargo build -p nrf1-cli
PYTHONPATH=impl/python/nrf_core_ref PATH=target/debug:$PATH \
  python3 -m pytest -q tests/differential/test_diff_cli_vs_python.py
```

## 3) Releases oficiais (tag → binários + checksums + SBOM + assinatura)

### 3.1 Versionamento e tag

- SemVer: `MAJOR.MINOR.PATCH` (ex.: `2.0.0`)
- Criar tag e publicar:
  ```bash
  git tag v2.0.0
  git push origin v2.0.0
  ```

### 3.2 Artefatos do release

O workflow `.github/workflows/release.yml` gera e publica, no GitHub Releases:
- Binários (`ai-nrf1`, `ubl`, `nrf1`) por OS
- `CHECKSUMS.sha256` + `CHECKSUMS.sha512`
- SBOMs em `dist/sbom/*` (best-effort)
- Assinaturas `cosign sign-blob` (keyless) para cada arquivo em `dist/`

## 4) Distribuição (LAB512 = este computador)

### 4.1 Nightly local (self-hosted runner)

O workflow `.github/workflows/nightly-lab512.yml` (opcional) roda em runner **self-hosted** e publica em disco local.

- Diretório padrão: `/opt/lab512/artifacts/nightly/<sha>/`
- Link “latest”: `/opt/lab512/artifacts/nightly/latest`

Para habilitar, registre este computador como **GitHub Actions self-hosted runner** e adicione a label `lab512`.
Depois, o workflow pode rodar via `workflow_dispatch` ou cron.

### 4.2 Exposição via Cloudflare Tunnel (opcional)

Isso é infra local (fora do repositório), mas o objetivo é publicar `.../latest/` atrás de Cloudflare Access.

## 5) Segurança e supply chain

- Checksums: `scripts/make_checksums.sh`
- SBOM: `scripts/sbom.sh` (usa `syft` quando disponível)
- Assinatura: `cosign` keyless (OIDC) no workflow de release
- Segredos: nunca commitar chaves privadas; vetores usam `tests/keys/` (gitignored)

## 6) Dia a dia (TL;DR)

1. `git checkout -b feat/<slug>`
2. Rodar checks locais (seção 2)
3. Commit + push + PR
4. CI verde + review
5. Merge
6. Release: `git tag vX.Y.Z && git push origin vX.Y.Z`

---

# MODULES Layer — Operations

## 7) Effect Bindings & Secrets

All IO bindings are declared in `product.json` → `io_bindings`.
Values prefixed with `env:` are resolved from environment variables at runtime.
**Secret values are never logged.**

| Binding Key | Example Value | Used By |
|---|---|---|
| `webhook.url` | `https://api.client.com/hooks` | `Effect::Webhook` |
| `webhook.hmac` | `env:WH_SEC` | HMAC signing (`X-UBL-Signature`) |
| `storage.dir` | `/var/lib/ubl/artifacts` | `Effect::WriteStorage` |
| `NODE_KEY` | `env:REGISTRY_SIGNING_KEY` | `Effect::AppendReceipt` (Ed25519 seed) |
| `partner.relay.url` | `https://relay.partner.net/ingest` | `Effect::RelayOut` |
| `OPENAI_GPT4O_MINI` | `env:OPENAI_API_KEY` | `Effect::InvokeLlm` |

### Resolution rules

1. Literal string → used as-is.
2. `env:VAR_NAME` → `std::env::var("VAR_NAME")`. Fails if unset.
3. Binding not found → `Err.Config.Invalid` (HTTP 400).

## 8) Idempotency

### In-memory (default)

`IdempotentExecutor<E>` wraps any executor. Dedup key:

```
run_key = blake3(capsule_id || step_id || effect_kind || discriminator)
```

Discriminator varies by effect (e.g., `url` for Webhook, `ticket_id` for Consent).

### Durable (production)

File-based KV store at `${STATE_DIR}/idem/<run_key>`:

- **TTL**: configurable (default 1h), checked by file mtime.
- **GC**: `IdempotencyStore::gc()` removes expired entries.
- **Concurrency**: single-writer per `run_key` (file create is atomic on POSIX).

### STATE_DIR layout

```
~/.ai-nrf1/state/<tenant>/
├── idem/                  # idempotency markers
├── permit-tickets/        # consent ticket JSON files
└── llm-cache/             # LLM response cache
```

## 9) Permit (Consent) Routes

### Ticket lifecycle

```
PENDING → ALLOW  (K-of-N approvals received)
PENDING → DENY   (explicit denial)
PENDING → EXPIRED (TTL exceeded, timeout_action applied)
```

### CLI commands

```bash
# List pending tickets
ls ~/.ai-nrf1/state/<tenant>/permit-tickets/

# Approve (operator)
ubl permit approve --ticket <id> --role ops --sig auto

# Deny (operator)
ubl permit deny --ticket <id> --role ops --sig auto
```

### HTTP endpoints (minimal)

| Method | Path | Body | Effect |
|---|---|---|---|
| `POST` | `/permit/:ticket_id/approve` | `{"role": "ops", "sig": "hex"}` | Add approval, check quorum |
| `POST` | `/permit/:ticket_id/deny` | `{"role": "ops", "sig": "hex"}` | Close as DENY |
| `GET` | `/permit/:ticket_id` | — | Read ticket status |

### Resume flow

When a ticket closes as ALLOW, the executor re-submits the capsule to the
runner (same `trace_id`), starting from the step after the one that returned
`REQUIRE`. A hop with `kind=permit` is appended to the receipt chain.

## 10) Error Codes & HTTP Mapping

All runtime errors follow the `Err.<Category>.<Detail>` convention.

| Code | HTTP | When |
|---|---|---|
| `Err.Canon.NotASCII` | 400 | Non-ASCII in required ASCII field |
| `Err.Canon.InvalidNRF` | 400 | Malformed NRF bytes |
| `Err.Canon.ParseFailed` | 400 | JSON/manifest parse error |
| `Err.Hdr.Expired` | 410 | `exp < now - clock_skew` |
| `Err.Hdr.MissingField` | 400 | Required header field absent |
| `Err.Seal.BadSignature` | 403 | Ed25519 verify failed |
| `Err.Seal.Missing` | 401 | No seal present |
| `Err.Auth.Unauthorized` | 401 | No credentials |
| `Err.Auth.Forbidden` | 403 | Insufficient permissions |
| `Err.Policy.Deny` | 422 | Policy rules failed → DENY |
| `Err.Policy.Require` | 422 | Policy rules failed → REQUIRE (consent needed) |
| `Err.Hop.BadChain` | 422 | Receipt `prev` mismatch or reordering |
| `Err.Hop.BadSignature` | 422 | Receipt signature invalid |
| `Err.Replay` | 409 | Duplicate `(src, nonce)` |
| `Err.Idempotency.Conflict` | 409 | Effect already executed for this run_key |
| `Err.Permit.Expired` | 410 | Consent ticket TTL exceeded |
| `Err.Permit.InvalidRole` | 422 | Approver role not in `required_roles` |
| `Err.IO.WebhookFailed` | 502 | Webhook POST failed after retries |
| `Err.IO.RelayFailed` | 502 | Relay POST failed after retries |
| `Err.IO.LlmFailed` | 502 | LLM provider call failed |
| `Err.Config.Invalid` | 400 | Invalid module config |
| `Err.Config.CapNotFound` | 404 | Capability not registered |
| `Err.Internal` | 500 | Unexpected internal error |

### JSON error response format

```json
{
  "error": {
    "code": "Err.Hop.BadChain",
    "status": 422,
    "message": "prev mismatch at hop 3"
  }
}
```

## 11) Runbook — Common Failures

### Webhook returns 5xx

- **Behavior**: exponential retry (100ms × 2^n), max 5 attempts.
- **After exhaustion**: `Err.IO.WebhookFailed` (502). Effect is NOT retried on pipeline re-run (idempotency).
- **Action**: check target service health; re-run pipeline with new `run_id` if needed.

### Relay returns 429

- **Behavior**: same retry policy as webhooks.
- **Action**: increase `clock_skew_sec` if timing-related; check partner rate limits.

### Receipt chain invalid after append

- **Code**: `Err.Hop.BadChain`
- **Cause**: `prev` field doesn't match blake3 of previous receipt payload.
- **Action**: inspect receipt chain with `ubl_capsule::receipt::verify_chain()`. Likely a bug in hop ordering.

### Consent ticket expired

- **Code**: `Err.Permit.Expired`
- **Behavior**: `timeout_action` from config applied (usually DENY).
- **Action**: increase `ttl_sec` in product config, or approve faster.

### LLM provider timeout

- **Code**: `Err.IO.LlmFailed`
- **Behavior**: no retry (LLM calls are expensive). Cached responses are served if available.
- **Action**: check provider status; verify `model_binding` resolves correctly.

### Idempotency conflict on retry

- **Code**: `Err.Idempotency.Conflict` (409)
- **Behavior**: effect skipped silently (correct behavior).
- **Action**: no action needed — this means the effect already ran successfully.

## 12) Dry-Run Mode

Run the pipeline without executing any effects:

```bash
ubl product run --manifest products/api-receipt-gateway/product.json \
                --input sample.json \
                --dry-run
```

Uses `LoggingExecutor` internally — modules execute (pure), effects are
logged but not dispatched. Useful for manifest validation and DX.

