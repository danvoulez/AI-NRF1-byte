# BASE — Layer 1: Ground Truth

> The fractal invariant lives here.
> `Value → ρ(Value) → NRF bytes → BLAKE3 → CID → Ed25519 sig → Receipt`

This layer defines the atom of the system. Everything above (modules, products)
is built by composing this atom at larger scales.

## Constitution

[CONSTITUTION.md](CONSTITUTION.md) — 11 articles defining:

1. ρ normalization (NFC, no BOM, no nulls in maps, sorted keys)
2. The fractal invariant (Value → CID → sig)
3. 7 types (Null, Bool, Int64, String, Bytes, Array, Map)
4. 4 artifacts (Receipt, Permit, Ghost, Capsule)
5. 3 acts (ATTEST, EVALUATE, TRANSACT)
6. 4 decisions (ALLOW, DENY, REQUIRE, GHOST)
7. 5 policy families (Existence, Compliance, Threshold, Provenance, Authorization)

## Specifications

All formal specs live in `specs/`:

| Spec | What |
|------|------|
| [specs/ai-nrf1-spec.md](specs/ai-nrf1-spec.md) | NRF1 binary encoding format |
| [specs/ai-nrf1-capsule-v1.md](specs/ai-nrf1-capsule-v1.md) | Capsule v1 (header, seal, receipts) |
| [specs/rho-to-nrf-transistor.md](specs/rho-to-nrf-transistor.md) | ρ → NRF pipeline |
| [specs/ai-nrf1-json-mapping.md](specs/ai-nrf1-json-mapping.md) | JSON ↔ NRF rules |
| [specs/ai-nrf1-llm-guide.md](specs/ai-nrf1-llm-guide.md) | LLM interaction guide |
| [specs/ai-nrf1-security-considerations.md](specs/ai-nrf1-security-considerations.md) | Security model |
| [specs/BASE-SPEC.md](specs/BASE-SPEC.md) | Full BASE layer spec |

## Design Documents

| Doc | Topic |
|-----|-------|
| [DECIMALS-AND-NUMBERS.md](DECIMALS-AND-NUMBERS.md) | No floats — how numbers work |
| [GHOST_LIFECYCLE.md](GHOST_LIFECYCLE.md) | Ghost artifact lifecycle |
| [SIRP_MIN_BASE.md](SIRP_MIN_BASE.md) | SIRP receipt chain format |
| [UBL_JSON.md](UBL_JSON.md) | UBL JSON document format |
| [STACK-BOUNDARIES.md](STACK-BOUNDARIES.md) | Layer boundary rules |

## Crates

| Crate | Role in the fractal |
|-------|---------------------|
| `impl/rust/nrf-core/` | Value → ρ(Value) → NRF bytes |
| `impl/rust/ubl_json_view/` | JSON ↔ Value (canonical) |
| `impl/rust/ubl_capsule/` | CID → sig → Receipt → Capsule |
| `impl/rust/signers/` | Ed25519 / Dilithium signing |
| `crates/runtime/` | Runtime attestation |
