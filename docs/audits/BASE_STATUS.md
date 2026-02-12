# BASE Status — Product‑Grade Checklist (ai‑nrf1 / ubl‑byte)

Date: 2026-02-10

This repo aims to make the **BASE** (canonical bytes) strong enough to support many products with **universal offline verification**.

## 0) What exists (inventory)

| Path | Purpose | Status |
|---|---|---|
| `impl/rust/nrf-core` | Canonical ai‑nrf1 codec + ρ normalization + hashing | OK |
| `impl/rust/ubl_json_view` | Deterministic JSON view (ai‑json‑nrf1) | OK |
| `impl/rust/ubl_capsule` | ubl‑capsule v1: `id`, `seal`, `receipts` (SIRP chain) | OK |
| `tools/ubl-cli` | Developer CLI (`ubl`) for capsule ops + vector tooling | OK |
| `cli/nrf1` | Legacy/compat CLI (`nrf1`) used by differential tests | OK |
| `bindings/wasm/ai-nrf1-wasm` | WASM: encode/decode/hash/normalize/verify canon | OK |
| `bindings/wasm/ubl-capsule-wasm` | WASM: seal + receipts chain verification | OK |
| `tests/vectors/capsule` | Signed capsule vectors + keyring + negative cases | OK |
| `tests/base_conformance` | “Constitutional” conformance tests | OK |
| `tests/differential` | Rust CLI ↔ Python reference differential tests | OK |
| `crates/ubl-replay` | Anti‑replay module (non‑canonical): (src, nonce) LRU+TTL | OK |
| `.github/workflows/ci.yml` | CI: fmt/clippy/tests + fuzz build + wasm + node smoke + python differential | OK |

## 1) Expiry (Err.Hdr.Expired)

- Implemented in `impl/rust/ubl_capsule/src/seal.rs` via `hdr.exp` (epoch‑nanos).
- Configurable skew via `VerifyOpts { allowed_skew_ns }` (default `0`).
- Evidence:
  - Unit tests in `impl/rust/ubl_capsule/src/seal.rs` (expired/future/skew/no‑exp).
  - Signed vectors in `tests/vectors/capsule/` including `capsule_expired*`.

## 2) Anti‑replay (non‑canonical module)

- Implemented as `crates/ubl-replay`:
  - API: `ReplayCache::check_and_insert(src, nonce, exp)`.
  - LRU capacity + TTL derived from `exp - now` (capped).
  - Deterministic tests via injected clock.

## 3) Offline verification via WASM

- `bindings/wasm/ubl-capsule-wasm` exports:
  - `verifySealBytes(capsuleBytes, pkBytes, allowedSkewNs)`
  - `verifyReceiptsChainBytes(capsuleBytes, keyring)`
- `bindings/wasm/ai-nrf1-wasm` exports:
  - `encode`, `decode`, `hashBytes`, `hashValue`, `canonicalCid`, `normalize`, `verify`
- Cross‑lang smoke:
  - `tests/wasm/node_smoke.cjs` loads committed vectors and verifies:
    - ACK/ASK/NACK seals
    - 2‑hop receipts chain using `keyring.json`
    - expired / expired‑skew / tamper negative cases

## 4) Vectors + tooling

- Canonical, signed vectors live in `tests/vectors/capsule/`.
- Local‑only private keys live in `tests/keys/` (gitignored).
- Commands:
  - `make vectors` (requires `tools/ubl-cli` build; regenerates from JSON + signs deterministically)
  - `make vectors-verify` (verifies vectors, including negative expectations)

## 5) How to run locally

### Rust tests

- `cargo test --workspace`

### Vectors

- `make vectors`
- `make vectors-verify`

### WASM node smoke

Requires `wasm-pack` and `node`:

- `cargo install wasm-pack`
- `bash tools/wasm/build_node_pkgs.sh`
- `node tests/wasm/node_smoke.cjs`

Note (macOS): if you have Homebrew Rust installed, `cargo build --target wasm32-unknown-unknown` may fail.
Use rustup toolchain first in PATH:

- `PATH="$HOME/.cargo/bin:$PATH" cargo build -p ubl-capsule-wasm --target wasm32-unknown-unknown`

## 6) CI (what it proves)

`.github/workflows/ci.yml` runs:

- Rust: `fmt`, `clippy`, base tests (locked)
- Fuzz: `cargo fuzz build` smoke
- WASM: `cargo build --target wasm32-unknown-unknown`
- WASM‑Node: `wasm-pack build` + `node tests/wasm/node_smoke.cjs`
- Python: differential property tests (Rust CLI vs Python ref)

## Commands executed (this run)

- `make vectors`
- `make vectors-verify`
- `cargo test --workspace`
- `PATH="$HOME/.cargo/bin:$PATH" cargo build -p ubl-capsule-wasm --target wasm32-unknown-unknown`
- `PATH="$HOME/.cargo/bin:$PATH" cargo build -p ai-nrf1-wasm --target wasm32-unknown-unknown`

