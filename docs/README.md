# AI-NRF1-byte — Documentation

> **This project is fractal.** One invariant repeats at every layer, every scale,
> every artifact. Understand the invariant and you understand the whole system.

## The Fractal Invariant

```
Value → ρ(Value) → NRF bytes → BLAKE3 → CID → Ed25519 sig → Receipt
```

This chain is the atom of the system. It appears everywhere:

- **A single field** is a Value. Normalize it (ρ), encode it (NRF), hash it (BLAKE3), get a CID.
- **A capsule** is a Value containing other Values. Same chain: normalize, encode, hash, sign, receipt.
- **A pipeline step** takes a Value, transforms it, produces a receipt. Same chain.
- **A full pipeline** chains receipts. Each receipt points to the previous. Same chain, one level up.
- **A product** is a manifest (Value) that configures a pipeline. Same chain, one more level up.

The fractal means: if you understand how one Value becomes a CID, you understand how the entire system produces verifiable audit trails.

## The Seven Types

Everything in the system is built from exactly 7 types:

| Tag | Type | Notes |
|-----|------|-------|
| 0x00 | Null | Absence. Removed from maps by ρ normalization. |
| 0x01 | Bool(false) | |
| 0x02 | Bool(true) | |
| 0x03 | Int64 | No floats. Money = cents. Percentages = basis points. |
| 0x04 | String | UTF-8, NFC-normalized, no BOM. |
| 0x05 | Bytes | Raw binary. |
| 0x06 | Array | Ordered sequence of Values. |
| 0x07 | Map | Sorted keys (BTreeMap). No duplicate keys. No null values. |

No floats. No unsigned integers. No special date type. Timestamps are Strings in RFC-3339 UTC.

## The Three Layers

The project has exactly three layers. Each layer is governed by a constitution.
Each layer applies the same fractal invariant at a different scale.

```
docs/
  base/         ← Layer 1: ground truth (encoding, hashing, signing, receipts)
  modules/      ← Layer 2: capabilities (pure functions that transform Values)
  products/     ← Layer 3: configuration (manifests that compose modules)
  errors/       ← Error system (central error station, code + hint)
  ops/          ← Operations (deploy, middleware, CLI)
  audits/       ← Historical audits and status reports
```

### Layer 1: BASE (ground truth)

**Constitution**: [base/CONSTITUTION.md](base/CONSTITUTION.md)

The fractal invariant lives here. BASE defines:

- **ρ normalization**: NFC strings, no BOM, no nulls in maps, sorted keys
- **NRF encoding**: canonical binary format (7 types, deterministic)
- **BLAKE3 hashing**: `b3:<hex>` CIDs
- **Ed25519 signing**: capsule seals, receipt signatures
- **4 artifacts**: Receipt (proof), Permit (consent), Ghost (redaction), Capsule (envelope)
- **3 acts**: ATTEST, EVALUATE, TRANSACT

Key specs in `base/specs/`:

| Spec | What |
|------|------|
| [ai-nrf1-spec.md](base/specs/ai-nrf1-spec.md) | NRF1 binary encoding format |
| [ai-nrf1-capsule-v1.md](base/specs/ai-nrf1-capsule-v1.md) | Capsule format (header, seal, receipts) |
| [rho-to-nrf-transistor.md](base/specs/rho-to-nrf-transistor.md) | ρ → NRF encoding pipeline |
| [ai-nrf1-json-mapping.md](base/specs/ai-nrf1-json-mapping.md) | JSON ↔ NRF mapping rules |
| [ai-nrf1-llm-guide.md](base/specs/ai-nrf1-llm-guide.md) | How LLMs should interact with NRF |

Crates: `impl/rust/nrf-core/`, `impl/rust/ubl_capsule/`, `impl/rust/ubl_json_view/`

### Layer 2: MODULES (capabilities)

**Constitution**: [modules/CONSTITUTION.md](modules/CONSTITUTION.md)

Modules are **pure functions** that transform Values. The fractal repeats:
each module takes a Value (env), applies its logic, returns a new Value + verdict + artifacts.
The runner wraps each step in a receipt. Same chain.

- **Trait**: `Capability` with `validate_config()` and `execute()`
- **Input**: `CapInput { env, config, assets, prev_receipts, meta }`
- **Output**: `CapOutput { new_env, verdict, artifacts, effects, metrics }`
- **Effects returned, not executed** — the runtime handles IO
- **4 decisions**: ALLOW, DENY, REQUIRE, GHOST

The 8 capability families:

| Module | Purpose |
|--------|---------|
| cap-intake | Normalize input (mapping, set) |
| cap-policy | Evaluate rules (EXIST, THRESHOLD, ALLOWLIST, NOT) |
| cap-permit | K-of-N consent |
| cap-enrich | Render (status-page, webhook, badge, ghost) |
| cap-transport | Deliver (webhook, relay) |
| cap-llm | LLM integration |
| cap-pricing | Pricing engine |
| cap-runtime | Certified runtime execution |

Design docs: [modules/DESIGN.md](modules/DESIGN.md), [modules/LAYERING.md](modules/LAYERING.md), [modules/OPERATIONS.md](modules/OPERATIONS.md)

Crates: `modules/cap-*/`, `crates/modules-core/`, `crates/module-runner/`

### Layer 3: PRODUCTS (configuration)

**Constitution**: [products/CONSTITUTION.md](products/CONSTITUTION.md)

Products are **configuration, not code**. A product is a JSON/YAML manifest that
composes modules into a pipeline. The fractal repeats: the manifest itself is a
Value that gets normalized, hashed, and signed.

Products live in **separate repos**. This repo provides:
1. Registry service (HTTP API)
2. Runner service (pipeline execution)
3. Product factory (`tools/product-spitter/`)

Design docs: [products/ARCHITECTURE.md](products/ARCHITECTURE.md), [products/FACTORY.md](products/FACTORY.md)

## Error System

Every error in the codebase flows through one canonical shape:

```json
{
  "ok": false,
  "error": {
    "code": "Err.NRF.InvalidMagic",
    "message": "expected 'nrf1' magic header",
    "hint": "Ensure the buffer starts with the 4-byte NRF magic: 0x6e726631",
    "status": 400
  }
}
```

The error taxonomy follows the fractal: `Err.<Layer>.<Detail>`

| Prefix | Layer | Crate |
|--------|-------|-------|
| Err.NRF.* | BASE | nrf-core (17 variants) |
| Err.Rho.* | BASE | nrf-core::rho (3 variants) |
| Err.JsonView.* | BASE | ubl_json_view (13 variants) |
| Err.Hop.* | BASE | ubl_capsule::receipt (5 variants) |
| Err.Seal.* | BASE | ubl_capsule::seal (6 variants) |
| Err.Runtime.* | BASE | runtime (7 variants) |
| Err.Ledger.* | BASE | ubl-storage (2 variants) |
| Err.Replay.* | BASE | ubl-replay (3 variants) |
| Err.Auth.* | BASE | ubl-auth (4 variants) |
| Err.Policy.* | MODULES | module-runner (2 variants) |
| Err.Permit.* | MODULES | module-runner (3 variants) |
| Err.Config.* | MODULES | module-runner (2 variants) |
| Err.IO.* | MODULES | module-runner (4 variants) |
| Err.UblJson.* | MODULES | ubl-json (1 variant) |

Central station: `crates/ubl-error/` — see [errors/LLM-FIRST-AUDIT.md](errors/LLM-FIRST-AUDIT.md)

## Crate Map

The fractal is visible in the crate layout:

```
impl/rust/                    ← BASE: the invariant itself
  nrf-core/                   Value → ρ(Value) → NRF bytes
  ubl_json_view/              JSON ↔ Value (canonical)
  ubl_capsule/                CID → sig → Receipt → Capsule
  signers/                    Ed25519 / Dilithium

crates/                       ← Shared infrastructure
  ubl-error/                  Central error station (code + hint)
  modules-core/               Capability trait (the module fractal)
  module-runner/              Pipeline runner (chains the fractal)
  runtime/                    Runtime attestation
  ubl-auth/                   Authentication
  ubl-storage/                NDJSON ledger (append-only)
  ubl-replay/                 Anti-replay cache
  permit/                     Consent primitives
  receipt/                    Receipt primitives
  ghost/                      Ghost (redaction) primitives

modules/                      ← MODULES: pure capability functions
  cap-intake/                 Normalize (the fractal's input gate)
  cap-policy/                 Decide (ALLOW/DENY/REQUIRE/GHOST)
  cap-permit/                 Consent (K-of-N)
  cap-enrich/                 Render (artifacts)
  cap-transport/              Deliver (effects)
  cap-llm/                    LLM integration
  cap-pricing/                Pricing
  cap-runtime/                Certified execution

services/                     ← PRODUCTS: HTTP surfaces
  registry/                   Pipeline API + receipt store
  tdln-ui/                    Mother product (Next.js)
  ui-template/                Product skeleton

tools/                        ← Developer tools
  ubl-cli/                    CLI (the fractal from the terminal)
  product-spitter/            Product factory
```

## Reading Order for LLMs

1. **This file** — understand the fractal
2. **[base/CONSTITUTION.md](base/CONSTITUTION.md)** — the 11 articles of ground truth
3. **[base/specs/ai-nrf1-spec.md](base/specs/ai-nrf1-spec.md)** — the encoding format
4. **[modules/CONSTITUTION.md](modules/CONSTITUTION.md)** — how capabilities compose
5. **[products/CONSTITUTION.md](products/CONSTITUTION.md)** — how products are born
6. **[errors/LLM-FIRST-AUDIT.md](errors/LLM-FIRST-AUDIT.md)** — every error, explained
7. **[modules/OPERATIONS.md](modules/OPERATIONS.md)** — runtime operations
