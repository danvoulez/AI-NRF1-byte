# Constitution of the Modules

*The laws governing how BASE crates are composed into reusable units.*
*A module that violates these laws is not a module — it is a hack.*

---

## Preamble

A Module is a reusable unit of behavior that plugs into the Base.
It does not invent new primitives. It does not bypass the fractal invariant.
It takes the sockets the Base provides and fills them with domain logic.

The Base is the terrain. Modules are the roads.
Products drive on the roads. But the roads do not move the terrain.

**The Base is always the parent.** A module does not start itself, does not
sign its own receipts, does not own its own database, and does not open its
own port. The Base starts it, gives it rules, gives it signing authority,
and receives its output. A module that runs independently of the Base is
not a module — it is a rogue process.

---

## Article I — What a Module Is

### Section 1.1 — Definition

A Module is a Rust crate (or set of crates) that:

1. Implements one or more Base traits (`PolicyEngine`, `Signer`, etc.)
2. Consumes and produces Base artifacts (Receipt, Permit, Ghost, Capsule)
3. Declares which Act(s) it serves (ATTEST, EVALUATE, TRANSACT)
4. Declares which Policy Family it belongs to (if it is a policy module)
5. Never touches NRF encoding directly — it delegates to `nrf-core`

### Section 1.2 — What a Module Is NOT

A Module is NOT:

- A product (modules don't have billing, onboarding, or UI)
- A fork of a Base crate (you extend, you don't replace)
- A standalone service (modules are guests of the Base, never independent)
- An alternative implementation of NRF (there is only one)
- A process that starts itself (the Base starts it)
- A process that signs its own receipts (the Base signs for it)
- A process that opens its own port (the Base owns the network)

---

## Article II — The Module Contract

Every module MUST satisfy these requirements to be admitted to the workspace.

### Section 2.1 — Trait Implementation

A module that provides policy logic MUST implement `PolicyEngine`:

```rust
pub trait PolicyEngine: Send + Sync {
    fn evaluate(&self, req: &EvalRequest) -> anyhow::Result<EvalResponse>;
    fn family(&self) -> PolicyFamily;
}
```

A module that provides signing logic MUST implement `Signer`:

```rust
pub trait Signer {
    fn sign_canon_hash(&self, blake3_hash: &[u8; 32]) -> Result<Vec<u8>>;
}
```

### Section 2.2 — Receipt Emission

Every module that performs an act MUST emit a Receipt.
The Receipt MUST be constructed using `crates/receipt::Receipt`.
The Receipt MUST pass `verify_integrity()` before storage.

### Section 2.3 — Ghost Discipline

If a module performs an act that has side effects, it MUST:

1. Create a Ghost(pending) BEFORE execution
2. Execute the act
3. Either promote the Ghost (linking to the final Receipt) or expire it

This is Write-Before-Execute. There are no exceptions.

### Section 2.4 — Permit Verification

If a module receives a Permit, it MUST call `verify_permit()` before
executing. If verification fails, the module MUST NOT proceed.
A module that executes without verifying a required permit is in violation.

---

## Article III — Module Categories

### Section 3.1 — Policy Modules

Policy modules implement one of the five families:

| Family | Example Module |
|--------|---------------|
| Existence | `policy-schema-validator` — JSON Schema + required fields |
| Compliance | `policy-eu-ai-act` — EU AI Act conformance checks |
| Threshold | `policy-credit-score` — numeric bounds checking |
| Provenance | `policy-sbom-verify` — SBOM signature chain validation |
| Authorization | `policy-rbac` — role-based access control |

A policy module:
- Takes an `EvalRequest` (policy_id, context_cid, input, pipeline_prev)
- Returns an `EvalResponse` (decision, reasoning_hint, reasoning_cid, rules_fired)
- NEVER mutates state. Policy is pure evaluation.

### Section 3.2 — Intake Modules

Intake modules adapt external data into the canonical pipeline:

| Shape | Example Module |
|-------|---------------|
| Document | `intake-pdf` — extract structured data from PDF |
| Event | `intake-webhook` — normalize webhook payloads |
| Transaction | `intake-stripe` — map Stripe events to TRANSACT acts |

An intake module:
- Receives raw external data
- Produces a canonical `body: nrf1::Value` and `body_cid`
- NEVER makes policy decisions. Intake is pure transformation.

### Section 3.3 — Enrichment Modules

Enrichment modules produce human-facing artifacts from receipts:

| Type | Example Module |
|------|---------------|
| Card URL | `enrich-card` — OpenGraph-compatible preview card |
| Badge | `enrich-badge` — SVG/PNG trust badge |
| Status Page | `enrich-status` — live verification status |
| PDF | `enrich-pdf` — printable certificate |
| Webhook | `enrich-webhook` — push notification on receipt |

An enrichment module:
- Takes a finalized Receipt as input
- Produces a derived artifact (URL, image, document)
- NEVER modifies the Receipt. Enrichment is read-only.

### Section 3.4 — Runtime Modules

Runtime modules provide certified execution environments:

| Type | Example Module |
|------|---------------|
| LLM | `runtime-llm` — deterministic LLM judgment (seed=0, temp=0) |
| WASM | `runtime-wasm` — sandboxed WASM execution |
| TEE | `runtime-tee` — trusted execution environment attestation |

A runtime module:
- Fills the `rt` field of the Receipt (binary_sha256, hal_ref, env, certs)
- Produces a `reasoning_cid` if applicable
- MUST be deterministic: same input → same output → same hash

---

## Article IV — Module Composition Rules

### Section 4.1 — Modules Compose via pipeline_prev

Modules do not call each other directly. They compose by emitting receipts
that reference prior receipts via `pipeline_prev`.

```
Module A emits Receipt_A (act: ATTEST)
Module B reads Receipt_A.receipt_cid
Module B emits Receipt_B (act: EVALUATE, pipeline_prev: [Receipt_A.receipt_cid])
```

This is the ONLY composition mechanism. There are no orchestrators,
no workflow engines, no message buses. Just CIDs pointing to CIDs.

### Section 4.2 — Modules Do Not Share Mutable State

Two modules MUST NOT write to the same database table, the same file,
or the same in-memory structure. The Receipt is the shared state.
If you need to pass data between modules, put it in a Receipt and
reference it by CID.

### Section 4.3 — Module Versioning

Every module MUST declare a version. When a module's behavior changes
in a way that would produce different receipts for the same input,
it MUST increment its major version. The `rt.version` field in the
Receipt records which version produced it.

---

## Article V — Module Admission

### Section 5.1 — Compilation

A module MUST compile as part of `cargo check --workspace`.
A module that breaks the workspace is immediately removed.

### Section 5.2 — Tests

A module MUST include at least:
- One test proving it produces a valid Receipt
- One test proving its Receipt passes `verify_integrity()`
- One test proving its CIDs are deterministic

### Section 5.3 — Documentation

A module MUST declare in its `Cargo.toml` or README:
- Which Act(s) it serves
- Which Policy Family it belongs to (if policy)
- Which Base traits it implements
- What its input and output types are

---

## Article VI — The Parent Law

**The Base is always the parent. A module is always the child.**

### Section 6.1 — What the Base Provides

The Base provides to every module, regardless of language or runtime:

| Resource | How |
|----------|-----|
| **Principles** | ρ normalization, fractal invariant, constitutional articles |
| **Rules** | Policy families, evaluation contracts, decision vocabulary |
| **Signing** | Ed25519 key — the module never holds a signing key |
| **Storage** | Receipt/Ghost/Permit persistence — the module never touches the DB |
| **Identity** | Issuer DID, RuntimeInfo, binary attestation |
| **Network** | The port, the tunnel, the URL — the module never binds a socket |

### Section 6.2 — Module Execution Model

A module executes in one of two modes:

**Compiled-in (Rust modules):** The module is a library crate linked into
the Base binary. It calls Base functions directly. No serialization, no
network, no process boundary. This is the default and preferred mode.

**Spawned (non-Rust modules):** The Base spawns the module as a child
process. Communication happens via stdin/stdout or Unix socket. The Base
sends the module an `AttestationRequest` and receives back a result.
The Base signs the receipt. The module never sees the signing key.

In both modes, the invariant holds: the Base starts it, the Base feeds it,
the Base signs for it, the Base stores the result.

### Section 6.3 — A Module That Runs Alone Is Not a Module

If a process can produce receipts without a running Base, it is not a module.
If a process holds its own signing key, it is not a module.
If a process writes directly to the database, it is not a module.
If a process opens its own port, it is not a module.

It may be useful software. But it is not a module.

---

## Article VII — Forbidden Patterns

### Section 7.1 — No Direct NRF Encoding

Modules MUST NOT call `nrf_core::encode` or `nrf_core::decode` directly
for the purpose of creating canonical artifacts. They MUST use the
Receipt/Permit/Ghost constructors which handle canonical encoding internally.

### Section 7.2 — No Signature Bypass

A module MUST NOT produce an artifact with `sig: None` and store it
as if it were signed. Unsigned artifacts are drafts. Only signed
artifacts are canonical.

### Section 7.3 — No Decision Fabrication

A module MUST NOT set `decision: "ALLOW"` on a Receipt without having
actually evaluated a policy. The decision field reflects what the
PolicyEngine returned, not what the module wishes were true.

### Section 7.4 — No Ghost Skipping

A module MUST NOT skip the Ghost step for acts with side effects.
"It's faster without WBE" is not a valid argument. Speed without
accountability is not speed — it is debt.

---

## Ratification

```
This constitution governs all code in the MODULE phase.
It builds upon and never contradicts the Constitution of the Base.
A module that satisfies this constitution is welcome.
A module that violates it is removed.
```
