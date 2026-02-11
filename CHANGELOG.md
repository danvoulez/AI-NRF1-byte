# Changelog

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
