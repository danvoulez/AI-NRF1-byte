# MODULES — Layer 2: Capabilities

> The fractal at the molecule scale.
> Each module takes a Value, transforms it, returns a new Value + verdict + artifacts.
> The runner wraps each step in a receipt. Same chain as BASE, one level up.

## Constitution

[CONSTITUTION.md](CONSTITUTION.md) — 7 articles defining:

1. A module is a pure function (no IO, no side effects)
2. The contract: `CapInput → CapOutput`
3. Categories: intake, policy, permit, enrich, transport, llm, pricing, runtime
4. Composition: pipeline steps chain via `pipeline_prev`
5. Admission: capabilities must pass `validate_config()`
6. Parent law: modules obey BASE constitutions
7. Forbidden patterns: no network calls, no file IO, no global state

## The 8 Capability Families

| Module | Fractal role | What it does |
|--------|-------------|--------------|
| cap-intake | Input gate | Normalize and map input Values |
| cap-policy | Decision | Evaluate rules → ALLOW / DENY / REQUIRE / GHOST |
| cap-permit | Consent | K-of-N human approval |
| cap-enrich | Artifacts | Render status pages, webhooks, badges, ghosts |
| cap-transport | Effects | Deliver via webhook, relay |
| cap-llm | Intelligence | LLM integration (model binding, prompts) |
| cap-pricing | Commerce | Pricing engine (SKU → price with rules) |
| cap-runtime | Execution | Certified runtime (WASM, sandboxed) |

## Design Documents

| Doc | Topic |
|-----|-------|
| [DESIGN.md](DESIGN.md) | Module system design decisions |
| [LAYERING.md](LAYERING.md) | How modules compose in pipelines |
| [CONTRACT.md](CONTRACT.md) | The Capability trait contract |
| [OPERATIONS.md](OPERATIONS.md) | Runtime operations (effects, idempotency, permits, error codes) |
| [NAMING.md](NAMING.md) | Module naming conventions |
| [GETTING-STARTED.md](GETTING-STARTED.md) | Getting started with modules |

## Crates

| Crate | Role |
|-------|------|
| `crates/modules-core/` | `Capability` trait, `CapInput`, `CapOutput`, `Verdict`, `Effect` |
| `crates/module-runner/` | Pipeline runner, effect dispatch, error codes (25 `Err.*` codes with hints) |
| `modules/cap-*/` | The 8 capability implementations |
