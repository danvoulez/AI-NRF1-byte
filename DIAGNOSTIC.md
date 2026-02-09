# DIAGNOSTIC — Full Crate Health Report
Generated: 2026-02-09

## Canon Status

**Invariant: one value => one byte stream => one hash => one decision**

| Check | Status |
|---|---|
| `encode_value` / `decode_value` only in `nrf-core` | **PASS** |
| `crates/nrf1` delegates to `nrf-core` | **PASS** |
| `impl/rust/ai-nrf1` delegates to `nrf-core` | **PASS** |
| No other crate has its own encoder/decoder | **PASS** |
| `cargo check --workspace` | **PASS** (0 errors, 0 warnings) |
| `cargo test --workspace` | **PASS** (12/12) |

---

## Crate-by-Crate Status

### TIER 1 — Canon (in workspace, compiles, tested)

| Crate | Path | Status | Notes |
|---|---|---|---|
| `nrf-core` | `impl/rust/nrf-core` | **OK** | Single source of truth. Serde on Value. BLAKE3 hashing. Depth/size limits. |
| `ai-nrf1` | `impl/rust/ai-nrf1` | **OK** | Facade over nrf-core + CBOR compat module. Roundtrip tests pass (9/9). |
| `nrf1` | `crates/nrf1` | **OK** | Facade over nrf-core. Backward compat for receipt/reasoning-bit/sdk-rs. |

### TIER 2 — Domain crates (in workspace, compiles)

| Crate | Path | Status | Notes |
|---|---|---|---|
| `receipt` | `crates/receipt` | **OK** | Ed25519 sign/verify over NRF bytes. Uses nrf1 facade. |
| `reasoning-bit` | `crates/reasoning-bit` | **OK** | LLM judgment attestation. Fixed ed25519-dalek v2 API. |
| `ai-nrf1-sdk` | `crates/sdk-rs` | **OK** | JSON->NRF canonical conversion + CID. |
| `ubl-json` | `crates/ubl-json` | **OK** | JSON schema -> NRF Value mapping. Fixed broken nrf1-core path. |
| `envelope` | `crates/envelope` | **OK** | X25519+XChaCha20-Poly1305 AEAD. Fixed x25519-dalek v2 + hkdf. |
| `ubl-transport` | `crates/ubl-transport` | **OK** | Ed25519 transport layer. |
| `ubl-policy` | `crates/ubl-policy` | **OK** | Policy evaluation stubs. |

### TIER 3 — Service layer (in workspace, compiles)

| Crate | Path | Status | Issues |
|---|---|---|---|
| `ubl-auth` | `crates/ubl-auth` | **WARN** | **Auth bypass**: roles from `x-roles` header (line 100). Anyone can claim any role. PoP verification works but is optional — if no x-did/x-signature/x-pubkey headers, request passes with header-supplied roles. |
| `ubl-model` | `crates/ubl-model` | **OK** | Postgres receipt CRUD via sqlx. |
| `ubl-storage` | `crates/ubl-storage` | **OK** | S3 put_bytes. Minimal but functional. |
| `registry` | `services/registry` | **WARN** | Two conflicting architectures: (1) `main.rs` is monolithic with inline SQL and uses `ubl_auth::AuthCtx`, (2) `routes/*.rs` + `middleware/rbac.rs` are dead code referencing `crate::state::AppState` which doesn't exist. Dead code uses `state.db.*`, `state.s3.*`, `state.cfg.*` — none of these fields exist. |

### TIER 4 — CLI (partially in workspace)

| Crate | Path | In workspace? | Status | Issues |
|---|---|---|---|---|
| `nrf1-cli` | `cli/nrf1` | **YES** | **OK** | Renamed pkg to avoid collision. Binary still named `nrf1`. |
| `ainrf1` | `cli/ainrf1` | **NO (excluded)** | **BROKEN** | Duplicate `anyhow` dep key. Duplicate `Conformance` enum variant. Python syntax (`or`, `endswith`, `format` not `format!`). `base64::decode` (removed in base64 0.22). Missing `Determinism` import. ~15 compile errors. |

### TIER 5 — impl/ crates (NOT in workspace)

| Crate | Path | In workspace? | Status | Issues |
|---|---|---|---|---|
| `ai_nrf1_receipts` | `impl/rust/receipts` | **NO** | **WARN** | Was referencing `ai_nrf1_core` (fixed to `nrf-core`). Has its own `ReceiptV1` struct (different from `crates/receipt::Receipt`). Two receipt models exist. |
| `signers` | `impl/rust/signers` | **NO** | **OK-ish** | Compiles standalone. Uses `reqwest::blocking` (sync HTTP). `from_pkcs8_pem` needs `pkcs8` feature on ed25519-dalek (present). Not in workspace because nothing in workspace depends on it yet. |
| `nrf1-cli` | `impl/rust/nrf1-cli` | **NO (excluded)** | **BROKEN** | Duplicate `#[derive(clap::Parser)]`. Duplicate `Cmd` variants (`Judge`, `Verify`, `ReceiptBytes` appear twice). Dangling `} => {` syntax. `format` not `format!`. Python f-string `f"b3:{b3_hex}"`. References `ai_nrf1_receipts::Receipt` methods that don't exist (`canonical_bytes_without_sig`, `sign_with_sk`, `verify_with_vk`, `verify_invariants`, `pre`, `finalize_allow`, `finalize_ghost`). Old ed25519-dalek v1 API (`SecretKey::generate`, `PublicKey`). ~30+ compile errors. |

### TIER 6 — Isolated / Orphaned

| Crate | Path | In workspace? | Status | Issues |
|---|---|---|---|---|
| `nrf1-cli` | `tools/nrf1-cli` | **NO (own workspace)** | **WARN** | Has `[workspace]` directive (isolated). References `ai_nrf1_core` and `nrf1_core` in `judge.rs` (stale imports). References `ai_nrf1_receipts::Receipt` with methods that don't exist on the actual struct. |
| (no crate) | `crates/storage` | **NO** | **ORPHAN** | Has `src/lib.rs` + `src/s3.rs` but NO `Cargo.toml`. `s3.rs` has a more complete `S3Store` than `ubl-storage` (supports `put_json` with public URL return, `put_zip`, custom endpoint/creds). This is the code that `routes/ghosts.rs` and `routes/receipts.rs` expect via `state.s3`. |

---

## Cross-Cutting Issues

### 1. TWO RECEIPT MODELS
- `crates/receipt::Receipt` — used by `main.rs`, has `body: Value`, `sig`, `receipt_cid`, `RuntimeInfo`
- `impl/rust/receipts::ReceiptV1` — used by `tools/nrf1-cli` and `impl/rust/nrf1-cli`, has `ghost: Option<GhostInfo>`, `chain: Option<ChainInfo>`, no sign/verify methods

These need to converge into one.

### 2. AUTH BYPASS (ubl-auth)
`crates/ubl-auth/src/lib.rs:100` — roles come from `x-roles` HTTP header. Any caller can set `x-roles: app_owner` and bypass all RBAC. PoP verification is optional (only checked if x-did + x-signature + x-pubkey are all present).

### 3. REGISTRY DEAD CODE
`services/registry/src/routes/` and `services/registry/src/middleware/` reference `crate::state::AppState` which doesn't exist as a module. These files are never compiled (not declared as `mod` in `main.rs`).

### 4. DUPLICATE S3 IMPLEMENTATIONS
- `crates/ubl-storage` — minimal `put_bytes` only
- `crates/storage/src/s3.rs` — richer `put_json` + `put_zip` with public URL return (orphaned, no Cargo.toml)

### 5. STALE CRATE REFERENCES
- `tools/nrf1-cli/src/judge.rs` → `ai_nrf1_core::hash::b3_hex` (doesn't exist)
- `tools/nrf1-cli/src/judge.rs` → `ai_nrf1_receipts::{Receipt, RuntimeMeta, Decision}` (Receipt exists but RuntimeMeta/Decision don't)
- `impl/rust/nrf1-cli` → `ai_nrf1_receipts` methods that don't exist
- `impl/rust/receipts/Cargo.toml` → was `ai_nrf1_core` (fixed to `nrf-core`)

### 6. NAME COLLISIONS
- **`nrf1-cli`** appears 3 times: `cli/nrf1` (pkg renamed), `impl/rust/nrf1-cli`, `tools/nrf1-cli`
- Only `cli/nrf1` is in the workspace

### 7. cli/ainrf1 — UNSALVAGEABLE IN CURRENT FORM
~15+ syntax/API errors including Python syntax mixed with Rust. Needs full rewrite.

---

## Recommended Bulk Fix Order

1. **Add `impl/rust/receipts` + `impl/rust/signers` to workspace** — fix their deps, make them compile
2. **Merge receipt models** — converge `ReceiptV1` and `Receipt` into one canonical receipt in `crates/receipt`
3. **Fix ubl-auth** — remove `x-roles` header trust; roles must come from DB or PoP claims only
4. **Promote `crates/storage/src/s3.rs`** — give it a Cargo.toml, merge with or replace `ubl-storage`
5. **Clean registry** — wire `state.rs` module, mount routes, delete dead code
6. **Rewrite `cli/ainrf1`** — from scratch using working crates
7. **Delete or archive** `impl/rust/nrf1-cli` and `tools/nrf1-cli` (superseded by `cli/nrf1`)
