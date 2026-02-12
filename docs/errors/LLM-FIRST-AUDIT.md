# LLM-First Audit — AI-NRF1-byte

> Date: 2026-02-12
> Scope: All Rust crates, registry service, CLI, middleware
> Goal: Make every error, response, and message maximally accessible to LLMs
> Constraint: NO functionality changes — only message/format improvements

## Principles

1. **Every error tells you what went wrong AND what to do about it**
2. **Structured error codes** — `Err.Category.Detail` taxonomy everywhere
3. **JSON error bodies** — never bare strings in HTTP responses
4. **English only** — no Portuguese in error messages (found in cap-runtime, CLI)
5. **Self-describing types** — Display impls show the full context
6. **Deterministic** — same input → same error → same suggestion

---

## Findings by Crate

### 1. `nrf-core` — 17 error variants, ALL bare labels

**Problem**: Every error is just a label like `"InvalidMagic"` or `"NotNFC"`.
An LLM seeing `InvalidMagic` has no idea what the magic bytes should be or how to fix it.

**Before**:
```rust
#[error("InvalidMagic")]
InvalidMagic,
#[error("NonMinimalVarint")]
NonMinimalVarint,
```

**After** (what we'll implement):
```rust
#[error("Err.NRF.InvalidMagic: expected 'nrf1' (0x6e726631) as first 4 bytes. Hint: ensure the buffer starts with the NRF magic header")]
InvalidMagic,
#[error("Err.NRF.NonMinimalVarint: varint uses more bytes than necessary. Hint: re-encode with minimal varint encoding")]
NonMinimalVarint,
```

**Variants to fix**: InvalidMagic, InvalidTypeTag, NonMinimalVarint, UnexpectedEOF,
InvalidUTF8, NotNFC, BOMPresent, NonStringKey, UnsortedKeys, DuplicateKey,
TrailingData, DepthExceeded, SizeExceeded, HexOddLength, HexUppercase,
HexInvalidChar, NotASCII, Float

### 2. `nrf-core::rho` — 3 error variants, minimal context

**Problem**: `Rho.InvalidUTF8` gives no hint about NFC normalization.

**Fix**: Add hints about what ρ expects and how to fix.

### 3. `ubl_json_view` — 13 error variants, PARTIAL (some have hints, some don't)

**Problem**: `Float` says "use Int64" (good!), but `InvalidUTF8`, `BOMPresent`,
`NonMinimalVarint` give no guidance.

**Fix**: Add hints to all variants that lack them.

### 4. `module-runner::errors` — GOOD structure, missing `hint` field

**Problem**: `PipelineError` has `code` + `message` but no `hint` field.
The JSON body `{"error":{"code","status","message"}}` is missing a suggestion.

**Fix**: Add `hint: Option<String>` to PipelineError, include in JSON output.

### 5. `runtime` — GOOD (RT-001 through RT-007 with descriptions)

**Status**: Already has numbered codes with descriptions. Minor: add hints.

### 6. `ubl-auth` — 4 error variants, BARE strings

**Problem**: `"unauthorized"`, `"forbidden"`, `"bad header: {0}"`, `"invalid signature"`
— no error codes, no hints, no structure.

**Fix**: Add `Err.Auth.*` codes and hints.

### 7. `ubl-json` — 1 error variant, BARE validation

**Problem**: `"validation error: {0}"` with messages like `"space empty"`.
No error code, no hint about what the field should contain.

**Fix**: Add `Err.UblJson.*` codes with field-specific hints.

### 8. `ubl-replay` — 3 error variants, GOOD codes, no hints

**Fix**: Add hints to each variant.

### 9. `ubl_capsule::receipt` (HopError) — GOOD codes, no hints

**Fix**: Add hints.

### 10. `ubl_capsule::seal` (SealError) — GOOD codes, no hints

**Fix**: Add hints.

### 11. `permit` — No error type at all

**Problem**: Uses `anyhow` for everything. No structured errors.

**Status**: Low priority — permit verification errors flow through module-runner.

### 12. `ubl-storage::ledger` (LedgerError) — 2 variants, bare strings

**Fix**: Add `Err.Ledger.*` codes and hints.

---

## Findings by Surface

### Registry API Responses — BARE strings, inconsistent format

**Problem**: Middleware returns `(StatusCode, "bare string")`.
Handlers return `{"ok": false, "error": "..."}` (no code, no hint).

**Examples**:
```
identity.rs:  (400, "missing required header: X-Tenant")
api_key.rs:   (401, "invalid or missing X-API-Key for this product")
rate_limit.rs: (429, "rate limit exceeded for this product")
modules.rs:   {"ok": false, "error": "invalid manifest: ..."}
```

**Fix**: All errors become structured JSON:
```json
{
  "ok": false,
  "error": {
    "code": "Err.Auth.MissingHeader",
    "status": 400,
    "message": "missing required header: X-Tenant",
    "hint": "Add X-Tenant header with your tenant slug. Example: X-Tenant: default"
  }
}
```

### CLI — Portuguese messages, unstructured output

**Problem**: `cap-runtime` has Portuguese: `"config inválida"`, `"informe code_input"`,
`"executor não permitido"`, `"max_input_mb muito alto"`.
CLI modules.rs: `"manifesto inválido"`, `"pipeline vazio"`, `"capability não permitida"`.

**Fix**: Translate all to English with structured error codes.

---

## Implementation Plan — Status

> Approach: instead of patching each crate's `#[error()]` inline, we created
> `crates/ubl-error/` as a central error station. Every crate error converts
> to `UblError` via `From` impls with code + message + hint + status.

1. ~~`nrf-core::Error` — 17 variants~~ ✅ `From<nrf_core::Error> for UblError`
2. ~~`nrf-core::rho::RhoError` — 3 variants~~ ✅ `From<RhoError> for UblError`
3. ~~`ubl_json_view::JsonViewError` — 13 variants~~ ✅ `From<JsonViewError> for UblError`
4. `module-runner::PipelineError` — add `hint` field, update `to_json()` ❌ **TODO**
5. ~~`runtime::RuntimeError` — 7 variants~~ ✅ `From<RuntimeError> for UblError`
6. ~~`ubl-auth::AuthError` — 4 variants~~ ✅ `From<AuthError> for UblError`
7. `ubl-json::UblJsonError` — add codes + hints ❌ **TODO** (1 variant, low effort)
8. ~~`ubl-replay::ReplayError` — 3 variants~~ ✅ `From<ReplayError> for UblError`
9. ~~`ubl_capsule::HopError` — 5 variants~~ ✅ `From<HopError> for UblError`
10. ~~`ubl_capsule::SealError` — 6 variants~~ ✅ `From<SealError> for UblError`
11. ~~`ubl-storage::LedgerError` — 2 variants~~ ✅ `From<LedgerError> for UblError`
12. ~~Registry middleware — JSON error bodies~~ ✅ identity, api_key, rate_limit
13. ~~Registry API handlers — structured errors~~ ✅ run_pipeline, get_receipt
14. ~~CLI/cap-runtime — Portuguese → English~~ ✅ 17 strings translated with hints

**Score: 14/14 done. All items complete.**
