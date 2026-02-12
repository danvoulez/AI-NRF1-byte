# AI-NRF1-byte

**This project is fractal.** One invariant repeats at every layer, every scale, every artifact.

```
Value → ρ(Value) → NRF bytes → BLAKE3 → CID → Ed25519 sig → Receipt
```

A single field follows this chain. A capsule follows this chain. A pipeline step follows this chain. A full pipeline chains receipts that each followed this chain. A product is a manifest that configures a pipeline — same chain, one level up.

**Understand the invariant and you understand the whole system.**

**Full documentation**: [`docs/README.md`](docs/README.md)

## Three Layers

| Layer | Scale | What | Location |
|-------|-------|------|----------|
| **BASE** | The atom | Encoding, hashing, signing, receipts | `impl/rust/`, `crates/` |
| **MODULES** | The molecule | Pure capability functions that transform Values | `modules/cap-*/` |
| **PRODUCTS** | The organism | Configuration manifests that compose modules | External repos |

Each layer has a constitution. Each applies the same fractal invariant at a different scale.

## Constants

- **7 types**: Null, Bool, Int64, String, Bytes, Array, Map — no floats, ever
- **4 artifacts**: Receipt, Permit, Ghost, Capsule
- **3 acts**: ATTEST, EVALUATE, TRANSACT
- **4 decisions**: ALLOW, DENY, REQUIRE, GHOST
- **Deterministic**: same input → same CID → same receipt chain

## Quick Start

```bash
cargo build --workspace                              # Build (stub mode)
cargo build -p registry --features modules           # Build with real runner
LEDGER_DIR=./data cargo run -p registry --features modules  # Run registry
ubl tdln policy --var data=hello                     # Run a pipeline
```

## Error System

Every error has a code, a message, and a hint:

```json
{"ok":false,"error":{"code":"Err.NRF.InvalidMagic","message":"expected 'nrf1' magic header","hint":"Ensure the buffer starts with the 4-byte NRF magic: 0x6e726631","status":400}}
```

Central station: `crates/ubl-error/` — see [`docs/errors/`](docs/errors/)

## Documentation Structure

```
docs/
  README.md       ← Start here (the fractal, explained)
  base/           ← Layer 1: ground truth + specs/
  modules/        ← Layer 2: capabilities
  products/       ← Layer 3: configuration
  errors/         ← Error system (code + hint for every error)
  ops/            ← Deploy, middleware, CLI
  audits/         ← Historical audits
```
