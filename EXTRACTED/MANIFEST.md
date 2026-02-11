# EXTRACTED — Staging Manifest

Files extracted from `MODULOS-INICIO.md` for review before placement.

| File | Bytes | Target Location | Status |
|------|-------|-----------------|--------|
| `code/modules-core--lib.rs` | 2120 | `crates/modules-core/src/lib.rs` — NEW crate (module contracts) | NEEDS POLISH |
| `code/cap-intake--lib.rs` | 3901 | `modules/cap-intake/src/lib.rs` — NEW crate (capability module) | NEEDS POLISH |
| `code/cap-policy--lib.rs` | 3578 | `modules/cap-policy/src/lib.rs` — NEW crate (capability module) | NEEDS POLISH |
| `code/cap-enrich--lib.rs` | 3744 | `modules/cap-enrich/src/lib.rs` — NEW crate (capability module) | NEEDS POLISH |
| `code/runner--runner.rs` | 2822 | `crates/module-runner/src/runner.rs` — NEW crate (NOT crates/runtime which is BASE) | NEEDS POLISH |
| `code/runner--manifest.rs` | 678 | `crates/module-runner/src/manifest.rs` — NEW crate | NEEDS POLISH |
| `code/runner--cap_registry.rs` | 933 | `crates/module-runner/src/cap_registry.rs` — NEW crate | NEEDS POLISH |
| `code/runner--effects.rs` | 343 | `crates/module-runner/src/effects.rs` — NEW crate | NEEDS POLISH |
| `code/runner--assets.rs` | 785 | `crates/module-runner/src/assets.rs` — NEW crate | NEEDS POLISH |
| `code/runner--finalize.rs` | 1265 | `crates/module-runner/src/finalize.rs` — NEW crate (ASPIRATIONAL: needs ubl_capsule API) | ASPIRATIONAL |
| `schemas/product.v1.json` | 796 | `schemas/product.v1.json` — already existed, identical | DONE (no-op) |
| `examples/product--api-receipt-gateway.json` | 1358 | `products/api-receipt-gateway/product.json` | PLACED |
| `docs/MODULES-DESIGN.md` | 25644 | `docs/MODULES-DESIGN.md` | PLACED + POLISHED |

## Notes

- **NEEDS POLISH**: Code extracted verbatim from chat. Needs: import fixes, trait adjustments (associated consts → methods for dyn dispatch), removal of `#[async_trait]` on non-async traits, Default impls, etc.
- **ASPIRATIONAL**: References types/APIs that don't exist yet in the codebase (e.g. `ubl_capsule::Capsule` struct). Keep as reference, don't compile yet.
- **crates/runtime** (BASE) is UNTOUCHED. The new runner lives in `crates/module-runner`.
- **crates/modules-core** is a NEW crate — the contract layer between runner and modules.
