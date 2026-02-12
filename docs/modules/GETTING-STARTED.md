# Getting Started with Modules

> The fractal at the molecule scale.
> Modules are **capabilities, not use cases**. A use case is just configuration (JSON) injected into a capability.

## The Anti-Pattern

The danger after BASE is ready: creating a module for every feature (`mod-email`, `mod-slack`, `mod-pdf-finra`). This is chaos.

The solution: **Capability Modules** (engines). The "use case" is just data (manifest + packs + templates) injected into the engine. You never write `mod-policy-compliance-banking`. You write `cap-policy` and load a compliance pack.

## The 8 Canonical Capabilities

| # | Module | Role in the fractal | What it does |
|---|--------|---------------------|--------------|
| 1 | **cap-intake** | Input gate | Transform dirty input (JSON, webhooks, docs) into canonical `env` (NRF Value). Uses a `MappingSpec` — no per-source modules. |
| 2 | **cap-policy** | Decision | Evaluate rules → ALLOW / DENY / REQUIRE / GHOST. Intelligence is in the **pack** (versioned, signed), not the module. |
| 3 | **cap-permit** | Consent | Resolve REQUIRE state. K-of-N human approval, TTL, Ed25519-signed Permits. |
| 4 | **cap-enrich** | Artifacts | Generate human/system artifacts: HTML status page, SVG badge, signed webhook, ghost (redacted receipt). Template-driven, not per-product. |
| 5 | **cap-transport** | Delivery | Deliver artifacts via webhook, relay, SIRP hop chain. |
| 6 | **cap-llm** | Intelligence | LLM integration. The LLM **suggests**, BASE **signs**. Output enters the pipeline as data, never as a final decision. |
| 7 | **cap-pricing** | Commerce | Pricing engine (SKU → price with rules). |
| 8 | **cap-runtime** | Execution | Certified runtime (WASM sandbox, attestation). |

## The Module Contract

Every module implements the same trait:

```rust
pub trait Capability: Send + Sync {
    fn validate_config(&self, config: &Value) -> Result<(), Error>;
    fn execute(&self, input: CapInput) -> Result<CapOutput, Error>;
}
```

### CapInput (what the module receives)

- **env** — NRF Value (the canonical state)
- **config** — JSON from the manifest step
- **assets** — CID → bytes resolver (packs, templates, schemas)
- **prev_receipts** — CIDs of previous pipeline steps
- **meta** — run_id, tenant, trace_id, timestamp

### CapOutput (what the module returns)

- **new_env** — updated NRF Value (or None if unchanged)
- **verdict** — ALLOW / DENY / REQUIRE / GHOST (or None)
- **artifacts** — generated blobs (HTML, SVG, JSON)
- **effects** — side-effects to execute (webhook, storage) — **returned, not executed**
- **metrics** — observability data

## Golden Rules

1. **Determinism by default** — anything time/random-dependent enters via the runtime context
2. **No loose parsing** — JSON view becomes canonical NRF before entering core logic
3. **No decisions in LLM** — LLM generates *data*, never verdict
4. **No module per use case** — use case = Config + Packs + Templates
5. **Everything is CID** — prompts, templates, policy packs = content-addressed
6. **Effects returned, not executed** — the runtime handles IO, retries, idempotency
7. **core.rs imports no IO** — pure logic only; `io.rs` is where the real world lives

## Module File Structure

```
modules/<cap-name>/
  Cargo.toml
  src/
    lib.rs          # public exports
    config.rs       # manifest config types + validation
    core.rs         # deterministic, pure logic (NO IO imports)
    io.rs           # real-world adapters (store/network/clock)
    errors.rs       # standardized errors
  tests/
    integration.rs  # end-to-end module tests
```

## How Products Compose Modules

With just these 8 capabilities, you build any product:

| Product | Pipeline (manifest) |
|---------|---------------------|
| API Gateway | intake(Transaction) → policy(RateLimit+Auth) → enrich(Webhook) |
| Model Passport | intake(Doc) → policy(EU-AI-Act) → enrich(Badge+StatusPage) |
| Underwriting | intake(Doc) → policy(Risk) → permit(K=2,N=3) → enrich(PDF) |
| LLM Engine | intake(Prompt) → policy(Exist) → llm(GPT-4o) → enrich(StatusPage) → transport |

The product is **configuration, not code**. If creating a product requires writing Rust, something is wrong with BASE or MODULES.

## Testing Requirements

Each module must have:

1. **Known-Answer Tests (KAT)** — input JSON + expected canonical NRF output
2. **Property tests** — invariants (e.g., intake: same input → same env; policy: same env+packs → same decision)
3. **Integration tests** — plug into the runner with a minimal manifest, run 1 pipeline

## Next Steps

1. Read [DESIGN.md](DESIGN.md) for module system design decisions
2. Read [LAYERING.md](LAYERING.md) for how modules compose in pipelines
3. Read [OPERATIONS.md](OPERATIONS.md) for runtime operations (effects, idempotency, permits)
4. Read [CONTRACT.md](CONTRACT.md) for the full Capability trait contract