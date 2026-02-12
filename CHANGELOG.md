# Changelog

## v1.1.0 — Canon Enforcement (BREAKING)

### Breaking Changes

- **Bytes in JSON**: The ONLY accepted form is now `{"$bytes": "<lowercase hex>"}`.
  Previously `"b3:<hex>"` (32 bytes), `"b64:<base64>"` (16/64 bytes), and
  `{"$bytes": "<base64>"}` (other) were used. All three are removed.
  - `ubl_json_view::to_json` always emits `{"$bytes": "<hex>"}`
  - `ubl_json_view::from_json` only accepts `{"$bytes": "<hex>"}`
  - WASM `encode`/`decode` use the same convention
  - **Migration**: replace `"b3:<hex>"` → `{"$bytes": "<hex>"}` and
    `"b64:<base64>"` → `{"$bytes": "<hex>"}` in stored JSON

- **Capsule `sign()` returns `Result`**: `ubl_capsule::seal::sign()` and
  `ubl_capsule::compute_id()` now return `Result` instead of bare values.
  Callers must handle the error (`.unwrap()` in tests, `?` in production).

- **Floats are a hard error**: `env.body` containing floats (`3.14`, `1e3`)
  or numbers outside i64 range now cause `sign()`/`compute_id()` to fail.
  Previously these were silently converted to strings, producing divergent hashes.

### Canon Enforcement

- **Canon 2,6**: `json_to_nrf_strict` rejects non-i64 numbers — no silent degradation
- **Canon 3**: All capsule values pass through `ρ(normalize)` before `encode`
- **Canon 4**: Single bytes convention across Rust, WASM, and Python: `{"$bytes":"<hex>"}`
- **Canon 6**: `ubl_json_view::from_json` rejects uppercase hex, odd-length hex, floats, BOM, non-NFC

### Tests

- 7 new hardening tests: float rejection, exponent rejection, u64 overflow,
  `$bytes` roundtrip, uppercase hex rejection, `b3:` prefix stays string
- All 200+ workspace tests pass

## v1.0.0 — BASE stable (ai-nrf1/ubl)

- Canon by **NRF bytes** (ai-nrf1): JSON is a derived view, hashes/signatures are deterministic.
- **Stable ID**: `id = blake3(nrf.encode(capsule \ {id, seal.sig, receipts[*].sig}))`.
- **Seal** with domain separation: signs `{domain, id, hdr, env}`.
- **Chained receipts**: auditable chain (BLAKE3), without touching the seal.
- **CLIs**: `ai-nrf1`, `ubl`, `nrf1` (encode/decode/hash/sign/verify/receipts).
- **WASM** offline verify + **Python differential** passing.
- **Registry** with health endpoints; module routes behind `--features modules`.
- **CI**: fmt, clippy, tests (BASE + modules), WASM, fuzz smoke, registry HTTP integration.
- **MODULES (experimental)**: `cap-intake`, `cap-policy`, `cap-permit`, `cap-enrich`, `cap-transport`, `cap-llm` — gated behind feature flag, API/ABI may change before v2.0.

## v0.3.9 (BASE sealed)
- Seal NRF‑1.1 Core: wire format, errors, ABNF, NFC/BOM, varint minimal
- Add JSON↔NRF import mapping (normative)
- Add Security Considerations
- Add LLM Pocket Guide
- Update README (positioning & differentiation)
- Scaffold AI Model Passport product & example E2E
